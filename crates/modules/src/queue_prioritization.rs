//! Queue Prioritization Engine Module
//!
//! This module provides intelligent job prioritization based on priority, SLA,
//! and fairness across tenants with support for preemption.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use hodei_core::{DomainError, JobId, Result};
use tokio::sync::RwLock;
use tracing::info;

use crate::sla_tracking::{SLALevel, SLATracker};

/// Job prioritization information
#[derive(Debug, Clone)]
pub struct PrioritizationInfo {
    pub job_id: JobId,
    pub base_priority: u8,     // User-specified priority (1-10)
    pub sla_level: SLALevel,   // SLA priority level
    pub tenant_id: String,     // Tenant for fair-share
    pub priority_score: f64,   // Calculated priority score
    pub can_preempt: bool,     // Can be preempted
    pub preemption_score: f64, // Score for preemption decisions
}

/// Preemption candidate
#[derive(Debug, Clone)]
pub struct PreemptionCandidate {
    pub job_id: JobId,
    pub current_job_id: JobId,
    pub priority_score: f64,
    pub tenant_id: String,
    pub estimated_waste: Duration,
}

/// Fair-share allocation
#[derive(Debug, Clone)]
pub struct FairShareAllocation {
    pub tenant_id: String,
    pub allocated_slots: u32,
    pub used_slots: u32,
    pub weight: f64,
    pub fairness_score: f64, // 0.0 to 1.0 (1.0 = perfectly fair)
}

/// Prioritization strategy
#[derive(Debug, Clone)]
pub enum PrioritizationStrategy {
    SLAFirst,      // SLA deadlines take precedence
    PriorityFirst, // User priority takes precedence
    FairShare,     // Equal allocation across tenants
    WeightedFair,  // Weighted allocation based on weights
    Hybrid,        // Combine multiple strategies
}

/// Preemption policy
#[derive(Debug, Clone)]
pub enum PreemptionPolicy {
    Never,           // Never preempt jobs
    Always,          // Always preempt for higher priority
    Conditional,     // Preempt if benefit exceeds threshold
    TenantProtected, // Don't preempt jobs from protected tenants
}

/// Prioritization statistics
#[derive(Debug, Clone)]
pub struct PrioritizationStats {
    pub total_prioritized: u64,
    pub preemptions_requested: u64,
    pub preemptions_executed: u64,
    pub average_priority_score: f64,
    pub fairness_variance: f64, // Lower is more fair
}

/// Queue prioritization engine
#[derive(Debug)]
pub struct QueuePrioritizationEngine {
    pub prioritized_jobs: Arc<RwLock<VecDeque<PrioritizationInfo>>>,
    pub tenant_allocations: Arc<RwLock<HashMap<String, FairShareAllocation>>>,
    pub sla_tracker: Arc<SLATracker>,
    pub strategy: PrioritizationStrategy,
    pub preemption_policy: PreemptionPolicy,
    pub max_jobs_per_tenant: u32,
    pub preemption_threshold: f64, // Minimum benefit to trigger preemption
    pub protected_tenants: Arc<RwLock<Vec<String>>>,
}

impl QueuePrioritizationEngine {
    pub fn new(sla_tracker: Arc<SLATracker>) -> Self {
        Self {
            prioritized_jobs: Arc::new(RwLock::new(VecDeque::new())),
            tenant_allocations: Arc::new(RwLock::new(HashMap::new())),
            sla_tracker,
            strategy: PrioritizationStrategy::Hybrid,
            preemption_policy: PreemptionPolicy::Conditional,
            max_jobs_per_tenant: 10,
            preemption_threshold: 0.1,
            protected_tenants: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_strategy(mut self, strategy: PrioritizationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_preemption_policy(mut self, policy: PreemptionPolicy) -> Self {
        self.preemption_policy = policy;
        self
    }

    pub fn with_max_jobs_per_tenant(mut self, max_jobs: u32) -> Self {
        self.max_jobs_per_tenant = max_jobs;
        self
    }

    pub fn with_preemption_threshold(mut self, threshold: f64) -> Self {
        self.preemption_threshold = threshold;
        self
    }

    pub async fn add_protected_tenant(&self, tenant_id: String) {
        let mut protected = self.protected_tenants.write().await;
        if !protected.contains(&tenant_id) {
            protected.push(tenant_id);
        }
    }

    /// Prioritize and enqueue a job
    pub async fn prioritize_job(
        &self,
        job_id: JobId,
        base_priority: u8,
        sla_level: SLALevel,
        tenant_id: String,
    ) -> PrioritizationInfo {
        let priority_score = self
            .calculate_priority_score(base_priority, &sla_level, &tenant_id)
            .await;
        let preemption_score = self.calculate_preemption_score(base_priority, &sla_level);

        let info = PrioritizationInfo {
            job_id,
            base_priority,
            sla_level: sla_level.clone(),
            tenant_id: tenant_id.clone(),
            priority_score,
            can_preempt: preemption_score > 0.5,
            preemption_score,
        };

        let mut queue = self.prioritized_jobs.write().await;

        // Insert job in priority order (highest first)
        let mut inserted = false;
        let mut insert_index = 0;

        for (index, existing_job) in queue.iter().enumerate() {
            if info.priority_score > existing_job.priority_score {
                insert_index = index;
                inserted = true;
                break;
            }
            insert_index = index + 1;
        }

        if inserted {
            queue.insert(insert_index, info.clone());
        } else {
            queue.push_back(info.clone());
        }

        info!(job_id = %job_id, priority_score, "Job prioritized and enqueued");

        // Update fair-share allocation
        self.update_fair_share_allocation(&tenant_id).await;

        info
    }

    /// Calculate priority score based on strategy
    async fn calculate_priority_score(
        &self,
        base_priority: u8,
        sla_level: &SLALevel,
        tenant_id: &str,
    ) -> f64 {
        let sla_weight = match self.strategy {
            PrioritizationStrategy::SLAFirst => 0.9,
            PrioritizationStrategy::Hybrid => 0.7,
            _ => 0.3,
        };

        let priority_weight = match self.strategy {
            PrioritizationStrategy::PriorityFirst => 0.7,
            PrioritizationStrategy::Hybrid => 0.2,
            _ => 0.1,
        };

        let fairness_weight = match self.strategy {
            PrioritizationStrategy::FairShare | PrioritizationStrategy::WeightedFair => 0.7,
            PrioritizationStrategy::Hybrid => 0.1,
            PrioritizationStrategy::SLAFirst | PrioritizationStrategy::PriorityFirst => 0.0,
        };

        // SLA component (higher SLA level = higher score)
        let sla_score = match sla_level {
            SLALevel::Critical => 100.0,
            SLALevel::High => 80.0,
            SLALevel::Medium => 60.0,
            SLALevel::Low => 40.0,
            SLALevel::BestEffort => 20.0,
        };

        // Base priority component (1-10)
        let priority_score = (base_priority as f64) * 10.0;

        // Fairness component
        let fairness_score = self.calculate_fairness_score(tenant_id).await;

        

        (sla_score * sla_weight)
            + (priority_score * priority_weight)
            + (fairness_score * fairness_weight * 100.0)
    }

    /// Calculate fairness score for a tenant
    async fn calculate_fairness_score(&self, tenant_id: &str) -> f64 {
        let allocations = self.tenant_allocations.read().await;
        let allocation = match allocations.get(tenant_id) {
            Some(a) => a,
            None => return 1.0, // New tenant gets maximum fairness score
        };

        // Score based on how much of the tenant's allocation is used
        // Underutilized tenants get higher scores
        let utilization = if allocation.allocated_slots > 0 {
            allocation.used_slots as f64 / allocation.allocated_slots as f64
        } else {
            0.0
        };

        // Inverse of utilization + fairness component
        1.0 - utilization + (1.0 - allocation.fairness_score)
    }

    /// Calculate preemption score
    fn calculate_preemption_score(&self, base_priority: u8, sla_level: &SLALevel) -> f64 {
        let sla_multiplier = match sla_level {
            SLALevel::Critical => 1.0,
            SLALevel::High => 0.8,
            SLALevel::Medium => 0.6,
            SLALevel::Low => 0.4,
            SLALevel::BestEffort => 0.2,
        };

        (base_priority as f64 / 10.0) * sla_multiplier
    }

    /// Update fair-share allocation for a tenant
    async fn update_fair_share_allocation(&self, tenant_id: &str) {
        let mut allocations = self.tenant_allocations.write().await;

        // Calculate totals before getting mutable reference
        let total_allocated: u32 = allocations.values().map(|a| a.allocated_slots).sum();
        let total_used_before: u32 = allocations.values().map(|a| a.used_slots).sum();

        let allocation =
            allocations
                .entry(tenant_id.to_string())
                .or_insert_with(|| FairShareAllocation {
                    tenant_id: tenant_id.to_string(),
                    allocated_slots: self.max_jobs_per_tenant,
                    used_slots: 0,
                    weight: 1.0,
                    fairness_score: 1.0,
                });

        allocation.used_slots = allocation.used_slots.saturating_add(1);

        // Calculate fairness score based on deviation from ideal allocation
        let total_allocated_after = total_allocated + self.max_jobs_per_tenant;
        let total_used_after = total_used_before + 1;

        if total_allocated_after > 0 {
            let ideal_share = allocation.allocated_slots as f64 / total_allocated_after as f64;
            let actual_share = allocation.used_slots as f64 / total_used_after as f64;

            allocation.fairness_score = 1.0 - (ideal_share - actual_share).abs();
        }
    }

    /// Get next job from queue (FIFO within same priority)
    pub async fn get_next_job(&self) -> Option<PrioritizationInfo> {
        let mut queue = self.prioritized_jobs.write().await;
        queue.pop_front()
    }

    /// Check for preemption candidates
    pub async fn check_preemption_candidates(&self, new_job_id: JobId) -> Vec<PreemptionCandidate> {
        if let PreemptionPolicy::Never = self.preemption_policy { return Vec::new() }

        let protected = self.protected_tenants.read().await;
        let mut candidates = Vec::new();

        let queue = self.prioritized_jobs.read().await;
        let new_job = queue.iter().find(|j| j.job_id == new_job_id);

        if let Some(new_job) = new_job {
            for (index, current_job) in queue.iter().enumerate() {
                // Skip if same tenant (we don't preempt our own jobs)
                if current_job.tenant_id == new_job.tenant_id {
                    continue;
                }

                // Skip if current job is from protected tenant
                if protected.contains(&current_job.tenant_id) {
                    continue;
                }

                // Check if new job has significantly higher priority
                if new_job.priority_score > current_job.priority_score {
                    let benefit = new_job.priority_score - current_job.priority_score;

                    if benefit > self.preemption_threshold {
                        let candidate = PreemptionCandidate {
                            job_id: new_job_id,
                            current_job_id: current_job.job_id,
                            priority_score: benefit,
                            tenant_id: current_job.tenant_id.clone(),
                            estimated_waste: Duration::from_secs(0), // Simplified
                        };
                        candidates.push(candidate);
                    }
                }
            }
        }

        candidates
    }

    /// Execute preemption
    pub async fn execute_preemption(&self, candidate: &PreemptionCandidate) -> Result<()> {
        info!(
            preempting_job = %candidate.current_job_id,
            with_job = %candidate.job_id,
            "Executing job preemption"
        );

        let mut queue = self.prioritized_jobs.write().await;

        // Find and remove the preempted job
        let mut new_queue = VecDeque::new();
        let mut preempted = false;

        while let Some(job) = queue.pop_front() {
            if job.job_id == candidate.current_job_id && !preempted {
                preempted = true;
                info!(job_id = %job.job_id, "Job preempted");
                continue;
            }
            new_queue.push_back(job);
        }

        *queue = new_queue;

        if !preempted {
            return Err(DomainError::NotFound("Preempted job not found".to_string()));
        }

        Ok(())
    }

    /// Get prioritization statistics
    pub async fn get_stats(&self) -> PrioritizationStats {
        let queue = self.prioritized_jobs.read().await;

        let total_prioritized = queue.len() as u64;
        let avg_score = if !queue.is_empty() {
            queue.iter().map(|j| j.priority_score).sum::<f64>() / queue.len() as f64
        } else {
            0.0
        };

        // Calculate fairness variance
        let allocations = self.tenant_allocations.read().await;
        let fairness_variance = if !allocations.is_empty() {
            let scores: Vec<f64> = allocations.values().map(|a| a.fairness_score).collect();
            let mean: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
            
            scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64
        } else {
            0.0
        };

        PrioritizationStats {
            total_prioritized,
            preemptions_requested: 0, // Would be tracked separately
            preemptions_executed: 0,  // Would be tracked separately
            average_priority_score: avg_score,
            fairness_variance,
        }
    }

    /// Get current queue state
    pub async fn get_queue_state(&self) -> Vec<PrioritizationInfo> {
        let queue = self.prioritized_jobs.read().await;
        queue.iter().cloned().collect()
    }

    /// Clear all priorities
    pub async fn clear(&self) {
        let mut queue = self.prioritized_jobs.write().await;
        queue.clear();

        let mut allocations = self.tenant_allocations.write().await;
        allocations.clear();
    }

    /// Remove a job from queue
    pub async fn remove_job(&self, job_id: &JobId) -> bool {
        let mut queue = self.prioritized_jobs.write().await;
        let initial_len = queue.len();

        queue.retain(|job| job.job_id != *job_id);

        let removed = queue.len() < initial_len;
        if removed {
            info!(job_id = %job_id, "Job removed from prioritization queue");
        }

        removed
    }
}

