//! Quota Enforcement Module
//!
//! This module provides quota enforcement at admission control, ensuring
//! resource allocation respects tenant limits and prevents overuse.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::identity_access::multi_tenancy_quota_manager::{
    MultiTenancyQuotaManager, QuotaDecision, QuotaError, ResourceRequest, TenantId,
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
#[derive(Debug, Clone)]
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
    QuotaError(#[from] crate::identity_access::multi_tenancy_quota_manager::QuotaError),
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
    ) -> Result<AdmissionDecision, QuotaError> {
        let start_time = std::time::Instant::now();

        self.stats.total_requests += 1;
        let tenant_id = request.tenant_id.clone();

        // First, check basic quota eligibility
        let quota_decision = self.quota_manager.check_quota(&tenant_id, &request).await?;

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
                if self.policy.preemption_enabled
                    && let Some(preempt_candidate) = self.find_preemption_candidate(&request).await
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

                AdmissionDecision {
                    allowed: false,
                    reason: Some(format!("Queued: {:?}", reason)),
                    estimated_wait: Some(*estimated_wait),
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
    pub async fn admit_request(&mut self, request: &ResourceRequest) -> Result<(), QuotaError> {
        let tenant_id = &request.tenant_id;

        // Double-check quota before admitting
        let quota_decision = self.quota_manager.check_quota(tenant_id, request).await?;

        match quota_decision {
            QuotaDecision::Allow { .. } => {
                // Allocate resources
                self.quota_manager
                    .allocate_resources(tenant_id, request)
                    .await?;

                info!("Admitted request for tenant {}", tenant_id);
                Ok(())
            }
            _ => Err(QuotaError::EnforcementError(
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
    ) -> Result<(), QuotaError> {
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
    async fn queue_request(&mut self, request: ResourceRequest) -> Result<Duration, QuotaError> {
        let tenant_id = request.tenant_id.clone();

        // Check queue size
        if let Some(queue) = self.queued_requests.get(&tenant_id)
            && queue.len() >= self.policy.max_queue_size {
                return Err(QuotaError::EnforcementError(
                    "Preemption candidate not found".to_string(),
                ));
            }

        // Create queue entry
        let queue_priority = match request.priority {
            crate::identity_access::multi_tenancy_quota_manager::JobPriority::Critical => 100,
            crate::identity_access::multi_tenancy_quota_manager::JobPriority::High => 80,
            crate::identity_access::multi_tenancy_quota_manager::JobPriority::Normal => 50,
            crate::identity_access::multi_tenancy_quota_manager::JobPriority::Low => 20,
            crate::identity_access::multi_tenancy_quota_manager::JobPriority::Batch => 10,
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
            .or_default();
        queue.push(queued_request);

        // Update stats
        self.stats.queued_requests += 1;

        // Estimate wait time (simplified)
        let estimated_wait = Duration::from_secs((queue.len() * 60) as u64);

        Ok(estimated_wait)
    }

    /// Process queued requests
    pub async fn process_queued_requests(&mut self) -> Result<(), QuotaError> {
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
            if let Some(queue) = self.queued_requests.get_mut(&tenant_id)
                && index < queue.len() {
                    queue.remove(index);
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

