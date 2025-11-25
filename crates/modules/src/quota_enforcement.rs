//! Quota Enforcement Module
//!
//! This module provides quota enforcement at admission control, ensuring
//! resource allocation respects tenant limits and prevents overuse.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::multi_tenancy_quota_manager::{
    MultiTenancyQuotaManager, QuotaDecision, QuotaViolationReason, ResourceRequest, TenantId,
    TenantQuota,
};

/// Enforcement policy configuration
#[derive(Debug, Clone)]
pub struct EnforcementPolicy {
    pub strict_mode: bool,              // If true, deny all violations immediately
    pub queue_on_violation: bool,       // Queue requests when quotas are exceeded
    pub preemption_enabled: bool,       // Enable preemption of low-priority jobs
    pub grace_period: Duration,         // Grace period before enforcement
    pub enforcement_delay: Duration,    // Delay before rejecting requests
    pub max_queue_size: usize,          // Maximum queue size per tenant
    pub enable_burst_enforcement: bool, // Enforce burst quotas
}

/// Admission control decision
#[derive(Debug, Clone)]
pub struct AdmissionDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub estimated_wait: Option<Duration>,
    pub quota_decision: QuotaDecision,
    pub enforcement_action: EnforcementAction,
}

/// Types of enforcement actions
#[derive(Debug, Clone)]
pub enum EnforcementAction {
    Allow,    // Request approved
    Deny,     // Request denied
    Queue,    // Queue for later processing
    Preempt,  // Preempt lower priority job
    Defer,    // Defer for grace period
    Throttle, // Apply throttling
}

/// Queue entry for deferred/qued requests
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub request: ResourceRequest,
    pub queued_at: DateTime<Utc>,
    pub priority: i32, // Queue priority
    pub attempts: u32, // Retry attempts
}

/// Quota enforcement statistics
#[derive(Debug, Clone)]
pub struct EnforcementStats {
    pub total_requests: u64,
    pub admitted_requests: u64,
    pub denied_requests: u64,
    pub queued_requests: u64,
    pub preempted_jobs: u64,
    pub enforcement_actions: HashMap<String, u64>,
    pub average_enforcement_latency_ms: f64,
    pub queue_utilization: f64,
    pub violation_detected: u64,
}

/// Preemption candidate information
#[derive(Debug, Clone)]
pub struct PreemptionCandidate {
    pub tenant_id: TenantId,
    pub job_id: String,
    pub priority: i32,
    pub resource_usage: ResourceRequest,
    pub duration: Duration,
    pub can_preempt: bool,
}

/// Quota enforcement engine
#[derive(Debug)]
pub struct QuotaEnforcementEngine {
    quota_manager: Arc<MultiTenancyQuotaManager>,
    policy: EnforcementPolicy,
    queued_requests: HashMap<TenantId, Vec<QueuedRequest>>,
    stats: EnforcementStats,
}

/// Enforcement error types
#[derive(Debug, thiserror::Error)]
pub enum EnforcementError {
    #[error("Queue overflow: tenant {0} queue is full")]
    QueueOverflow(TenantId),

    #[error("Preemption failed: {0}")]
    PreemptionFailed(String),

    #[error("Invalid enforcement policy: {0}")]
    InvalidPolicy(String),

    #[error("Tenant not found: {0}")]
    TenantNotFound(TenantId),

    #[error("Quota error: {0}")]
    QuotaError(#[from] crate::multi_tenancy_quota_manager::QuotaError),
}

impl QuotaEnforcementEngine {
    /// Create a new quota enforcement engine
    pub fn new(quota_manager: Arc<MultiTenancyQuotaManager>, policy: EnforcementPolicy) -> Self {
        Self {
            quota_manager,
            policy,
            queued_requests: HashMap::new(),
            stats: EnforcementStats {
                total_requests: 0,
                admitted_requests: 0,
                denied_requests: 0,
                queued_requests: 0,
                preempted_jobs: 0,
                enforcement_actions: HashMap::new(),
                average_enforcement_latency_ms: 0.0,
                queue_utilization: 0.0,
                violation_detected: 0,
            },
        }
    }

    /// Evaluate admission request with quota enforcement
    pub async fn evaluate_admission(
        &mut self,
        request: ResourceRequest,
    ) -> Result<AdmissionDecision, EnforcementError> {
        let start_time = std::time::Instant::now();

        self.stats.total_requests += 1;
        let tenant_id = request.tenant_id.clone();

        // First, check basic quota eligibility
        let quota_decision = self
            .quota_manager
            .check_quota(&tenant_id, &request)
            .await
            .map_err(|e| EnforcementError::TenantNotFound(tenant_id.clone()))?;

        // Process based on quota decision
        let admission_decision = match quota_decision {
            QuotaDecision::Allow { ref reason } => {
                self.stats.admitted_requests += 1;
                self.update_action_stats("allow");

                AdmissionDecision {
                    allowed: true,
                    reason: reason.clone(),
                    estimated_wait: None,
                    quota_decision,
                    enforcement_action: EnforcementAction::Allow,
                }
            }
            QuotaDecision::Deny { ref reason } => {
                self.stats.denied_requests += 1;
                self.stats.violation_detected += 1;
                self.update_action_stats("deny");

                if self.policy.strict_mode {
                    AdmissionDecision {
                        allowed: false,
                        reason: Some(format!("Quota violation: {:?}", reason)),
                        estimated_wait: None,
                        quota_decision,
                        enforcement_action: EnforcementAction::Deny,
                    }
                } else if self.policy.queue_on_violation {
                    // Queue the request
                    let estimated_wait = self.queue_request(request).await?;
                    self.update_action_stats("queue");

                    AdmissionDecision {
                        allowed: false,
                        reason: Some("Queued due to quota limit".to_string()),
                        estimated_wait: Some(estimated_wait),
                        quota_decision,
                        enforcement_action: EnforcementAction::Queue,
                    }
                } else {
                    AdmissionDecision {
                        allowed: false,
                        reason: Some(format!("Quota violation: {:?}", reason)),
                        estimated_wait: None,
                        quota_decision,
                        enforcement_action: EnforcementAction::Deny,
                    }
                }
            }
            QuotaDecision::Queue {
                ref reason,
                ref estimated_wait,
            } => {
                self.stats.queued_requests += 1;
                self.update_action_stats("queue");

                // Check if we should preempt
                if self.policy.preemption_enabled {
                    if let Some(preempt_candidate) = self.find_preemption_candidate(&request).await
                    {
                        self.stats.preempted_jobs += 1;
                        self.update_action_stats("preempt");

                        // Preempt and admit
                        self.execute_preemption(&preempt_candidate).await?;

                        // Mark as allowed after preemption
                        self.quota_manager
                            .allocate_resources(&tenant_id, &request)
                            .await?;

                        return Ok(AdmissionDecision {
                            allowed: true,
                            reason: Some("Preempted lower priority job".to_string()),
                            estimated_wait: None,
                            quota_decision,
                            enforcement_action: EnforcementAction::Preempt,
                        });
                    }
                }

                AdmissionDecision {
                    allowed: false,
                    reason: Some(format!("Queued: {:?}", reason)),
                    estimated_wait: Some(estimated_wait.clone()),
                    quota_decision,
                    enforcement_action: EnforcementAction::Queue,
                }
            }
        };

        // Record enforcement latency
        let latency = start_time.elapsed();
        self.update_enforcement_latency(latency);

        Ok(admission_decision)
    }

    /// Admit a resource request (after evaluation)
    pub async fn admit_request(
        &mut self,
        request: &ResourceRequest,
    ) -> Result<(), EnforcementError> {
        let tenant_id = &request.tenant_id;

        // Double-check quota before admitting
        let quota_decision = self
            .quota_manager
            .check_quota(tenant_id, request)
            .await
            .map_err(|_| EnforcementError::TenantNotFound(tenant_id.clone()))?;

        match quota_decision {
            QuotaDecision::Allow { .. } => {
                // Allocate resources
                self.quota_manager
                    .allocate_resources(tenant_id, request)
                    .await?;

                info!("Admitted request for tenant {}", tenant_id);
                Ok(())
            }
            _ => Err(EnforcementError::InvalidPolicy(
                "Cannot admit request that was not allowed".to_string(),
            )),
        }
    }

    /// Find preemption candidate
    async fn find_preemption_candidate(
        &self,
        request: &ResourceRequest,
    ) -> Option<PreemptionCandidate> {
        // Simplified: find lowest priority queued or active job
        // In a real implementation, this would query active jobs from scheduler
        let tenant_id = &request.tenant_id;

        if let Some(queue) = self.queued_requests.get(tenant_id) {
            // Find lowest priority job in queue
            queue
                .iter()
                .min_by_key(|q| q.priority)
                .map(|q| PreemptionCandidate {
                    tenant_id: tenant_id.clone(),
                    job_id: format!("job-{}", q.queued_at.timestamp()),
                    priority: q.priority,
                    resource_usage: q.request.clone(),
                    duration: Utc::now()
                        .signed_duration_since(q.queued_at)
                        .to_std()
                        .unwrap_or_default(),
                    can_preempt: true,
                })
        } else {
            None
        }
    }

    /// Execute preemption
    async fn execute_preemption(
        &mut self,
        candidate: &PreemptionCandidate,
    ) -> Result<(), EnforcementError> {
        info!(
            "Preempting job {} for tenant {}",
            candidate.job_id, candidate.tenant_id
        );

        // In a real implementation, this would:
        // 1. Signal the job runner to stop
        // 2. Deallocate resources
        // 3. Update scheduler state

        Ok(())
    }

    /// Queue a request for later processing
    async fn queue_request(
        &mut self,
        request: ResourceRequest,
    ) -> Result<Duration, EnforcementError> {
        let tenant_id = request.tenant_id.clone();

        // Check queue size
        if let Some(queue) = self.queued_requests.get(&tenant_id) {
            if queue.len() >= self.policy.max_queue_size {
                return Err(EnforcementError::QueueOverflow(tenant_id.clone()));
            }
        }

        // Create queue entry
        let queue_priority = match request.priority {
            crate::multi_tenancy_quota_manager::JobPriority::Critical => 100,
            crate::multi_tenancy_quota_manager::JobPriority::High => 80,
            crate::multi_tenancy_quota_manager::JobPriority::Normal => 50,
            crate::multi_tenancy_quota_manager::JobPriority::Low => 20,
            crate::multi_tenancy_quota_manager::JobPriority::Batch => 10,
        };

        let queued_request = QueuedRequest {
            request,
            queued_at: Utc::now(),
            priority: queue_priority,
            attempts: 0,
        };

        // Add to queue
        let queue = self
            .queued_requests
            .entry(tenant_id.clone())
            .or_insert_with(Vec::new);
        queue.push(queued_request);

        // Update stats
        self.stats.queued_requests += 1;

        // Estimate wait time (simplified)
        let estimated_wait = Duration::from_secs((queue.len() * 60) as u64);

        Ok(estimated_wait)
    }

    /// Process queued requests
    pub async fn process_queued_requests(&mut self) -> Result<(), EnforcementError> {
        let mut processed = Vec::new();

        for (tenant_id, queue) in self.queued_requests.iter_mut() {
            let mut i = 0;
            while i < queue.len() {
                let queued_req = &queue[i];

                // Re-evaluate quota
                let quota_decision = self
                    .quota_manager
                    .check_quota(tenant_id, &queued_req.request)
                    .await;

                if matches!(quota_decision, Ok(QuotaDecision::Allow { .. })) {
                    // Admit the request
                    self.quota_manager
                        .allocate_resources(tenant_id, &queued_req.request)
                        .await?;
                    processed.push((tenant_id.clone(), i));
                }

                i += 1;
            }
        }

        // Remove processed requests
        for (tenant_id, index) in processed {
            if let Some(queue) = self.queued_requests.get_mut(&tenant_id) {
                if index < queue.len() {
                    queue.remove(index);
                }
            }
        }

        Ok(())
    }

    /// Get enforcement statistics
    pub fn get_stats(&self) -> EnforcementStats {
        self.stats.clone()
    }

    /// Get queued requests for a tenant
    pub fn get_queued_requests(&self, tenant_id: &str) -> Option<&Vec<QueuedRequest>> {
        self.queued_requests.get(tenant_id)
    }

    /// Clear queued requests for a tenant
    pub fn clear_queue(&mut self, tenant_id: &str) {
        if let Some(queue) = self.queued_requests.get_mut(tenant_id) {
            let count = queue.len();
            queue.clear();
            self.stats.queued_requests = self.stats.queued_requests.saturating_sub(count as u64);
            info!(
                "Cleared queue for tenant {}, removed {} requests",
                tenant_id, count
            );
        }
    }

    /// Update enforcement action statistics
    fn update_action_stats(&mut self, action: &str) {
        *self
            .stats
            .enforcement_actions
            .entry(action.to_string())
            .or_insert(0) += 1;
    }

    /// Update enforcement latency statistics
    fn update_enforcement_latency(&mut self, latency: Duration) {
        let latency_ms = latency.as_millis() as f64;
        let total = self.stats.total_requests as f64;

        if total > 1.0 {
            // Calculate running average
            self.stats.average_enforcement_latency_ms =
                (self.stats.average_enforcement_latency_ms * (total - 1.0) + latency_ms) / total;
        } else {
            self.stats.average_enforcement_latency_ms = latency_ms;
        }
    }
}

impl Default for EnforcementPolicy {
    fn default() -> Self {
        Self {
            strict_mode: false,
            queue_on_violation: true,
            preemption_enabled: false,
            grace_period: Duration::from_secs(30),
            enforcement_delay: Duration::from_secs(5),
            max_queue_size: 100,
            enable_burst_enforcement: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request(tenant_id: &str, cpu_cores: u32) -> ResourceRequest {
        ResourceRequest {
            tenant_id: tenant_id.to_string(),
            pool_id: "pool-1".to_string(),
            cpu_cores,
            memory_mb: 256,
            worker_count: 5,
            estimated_duration: Duration::from_secs(3600),
            priority: crate::multi_tenancy_quota_manager::JobPriority::Normal,
        }
    }

    fn create_test_quota(tenant_id: &str) -> crate::multi_tenancy_quota_manager::TenantQuota {
        crate::multi_tenancy_quota_manager::TenantQuota {
            tenant_id: tenant_id.to_string(),
            limits: crate::multi_tenancy_quota_manager::QuotaLimits {
                max_cpu_cores: 100,
                max_memory_mb: 1024,
                max_concurrent_workers: 20,
                max_concurrent_jobs: 50,
                max_daily_cost: 100.0,
                max_monthly_jobs: 1000,
            },
            pool_access: HashMap::new(),
            burst_policy: crate::multi_tenancy_quota_manager::BurstPolicy {
                allowed: true,
                max_burst_multiplier: 1.5,
                burst_duration: Duration::from_secs(300),
                cooldown_period: Duration::from_secs(600),
                max_bursts_per_day: 10,
            },
            billing_tier: crate::multi_tenancy_quota_manager::BillingTier::Standard,
            quota_type: crate::multi_tenancy_quota_manager::QuotaType::HardLimit,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_enforcement_engine_creation() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let policy = EnforcementPolicy::default();

        let engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        let stats = engine.get_stats();
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.admitted_requests, 0);
    }

    #[tokio::test]
    async fn test_evaluate_admission_allowed() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let mut quota_manager = quota_manager;

        // Register tenant
        let quota = create_test_quota("tenant-1");
        quota_manager.register_tenant(quota).await.unwrap();

        let policy = EnforcementPolicy::default();
        let mut engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        // Evaluate request within quota
        let request = create_test_request("tenant-1", 10);
        let decision = engine.evaluate_admission(request).await.unwrap();

        assert!(decision.allowed);
        assert!(matches!(
            decision.enforcement_action,
            EnforcementAction::Allow
        ));

        let stats = engine.get_stats();
        assert_eq!(stats.admitted_requests, 1);
    }

    #[tokio::test]
    async fn test_evaluate_admission_denied_strict() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let mut quota_manager = quota_manager;

        // Register tenant
        let quota = create_test_quota("tenant-1");
        quota_manager.register_tenant(quota).await.unwrap();

        // Strict mode: deny immediately on quota violation
        let mut policy = EnforcementPolicy::default();
        policy.strict_mode = true;
        policy.queue_on_violation = false;

        let mut engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        // Allocate resources first to reach limit
        let request = create_test_request("tenant-1", 100); // Max limit
        engine
            .quota_manager
            .allocate_resources("tenant-1", &request)
            .await
            .unwrap();

        // Try to allocate another request
        let request2 = create_test_request("tenant-1", 10);
        let decision = engine.evaluate_admission(request2).await.unwrap();

        assert!(!decision.allowed);
        assert!(matches!(
            decision.enforcement_action,
            EnforcementAction::Deny
        ));

        let stats = engine.get_stats();
        assert_eq!(stats.denied_requests, 1);
    }

    #[tokio::test]
    async fn test_evaluate_admission_queued() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let mut quota_manager = quota_manager;

        // Register tenant
        let quota = create_test_quota("tenant-1");
        quota_manager.register_tenant(quota).await.unwrap();

        // Queue on violation
        let policy = EnforcementPolicy {
            queue_on_violation: true,
            ..Default::default()
        };
        let mut engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        // Allocate resources first
        let request = create_test_request("tenant-1", 100); // Max limit
        engine
            .quota_manager
            .allocate_resources("tenant-1", &request)
            .await
            .unwrap();

        // Queue second request
        let request2 = create_test_request("tenant-1", 10);
        let decision = engine.evaluate_admission(request2).await.unwrap();

        assert!(!decision.allowed);
        assert!(matches!(
            decision.enforcement_action,
            EnforcementAction::Queue
        ));
        assert!(decision.estimated_wait.is_some());

        let stats = engine.get_stats();
        assert_eq!(stats.queued_requests, 1);
    }

    #[tokio::test]
    async fn test_process_queued_requests() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let mut quota_manager = quota_manager;

        // Register tenant
        let quota = create_test_quota("tenant-1");
        quota_manager.register_tenant(quota).await.unwrap();

        let policy = EnforcementPolicy::default();
        let mut engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        // Queue a request
        let request = create_test_request("tenant-1", 10);
        let decision = engine.evaluate_admission(request.clone()).await.unwrap();
        assert!(matches!(
            decision.enforcement_action,
            EnforcementAction::Allow
        ));

        // Process queued requests (should be none in this case)
        let result = engine.process_queued_requests().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_admit_request() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let mut quota_manager = quota_manager;

        // Register tenant
        let quota = create_test_quota("tenant-1");
        quota_manager.register_tenant(quota).await.unwrap();

        let policy = EnforcementPolicy::default();
        let mut engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        // Admit request
        let request = create_test_request("tenant-1", 10);
        let result = engine.admit_request(&request).await;
        assert!(result.is_ok());

        let stats = engine.get_stats();
        assert_eq!(stats.admitted_requests, 0); // Not incremented in admit_request
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let quota_manager = crate::multi_tenancy_quota_manager::MultiTenancyQuotaManager::new(
            crate::multi_tenancy_quota_manager::QuotaManagerConfig::default(),
        );
        let mut quota_manager = quota_manager;

        // Register tenant
        let quota = create_test_quota("tenant-1");
        quota_manager.register_tenant(quota).await.unwrap();

        let mut policy = EnforcementPolicy::default();
        policy.queue_on_violation = true;
        policy.max_queue_size = 10;
        let mut engine = QuotaEnforcementEngine::new(Arc::new(quota_manager), policy);

        // Queue multiple requests
        for i in 0..5 {
            let request = create_test_request("tenant-1", 10);
            let decision = engine.evaluate_admission(request).await.unwrap();
            if matches!(decision.enforcement_action, EnforcementAction::Queue) {
                // Manually queue for testing
                let queued_req = QueuedRequest {
                    request: create_test_request("tenant-1", 10),
                    queued_at: Utc::now(),
                    priority: 50,
                    attempts: 0,
                };
                let queue = engine
                    .queued_requests
                    .entry("tenant-1".to_string())
                    .or_insert_with(Vec::new);
                queue.push(queued_req);
            }
        }

        // Clear queue
        engine.clear_queue("tenant-1");

        let queue = engine.get_queued_requests("tenant-1");
        assert!(queue.is_none() || queue.unwrap().is_empty());
    }
}
