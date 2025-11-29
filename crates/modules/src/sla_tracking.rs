//! SLA Tracking System Module
//!
//! This module provides SLA tracking for queued jobs with deadline monitoring,
//! violation alerts, priority adjustment, and compliance reporting.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use hodei_core::JobId;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// SLA tracking for a job
#[derive(Debug, Clone)]
pub struct SLAInfo {
    pub job_id: JobId,
    pub deadline: DateTime<Utc>,
    pub sla_level: SLALevel,
    pub priority_boost: u8, // Additional priority boost for at-risk jobs
    pub created_at: DateTime<Utc>,
}

/// SLA levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SLALevel {
    Critical,   // < 5 minutes
    High,       // < 15 minutes
    Medium,     // < 1 hour
    Low,        // < 4 hours
    BestEffort, // No specific SLA
}

/// SLA status
#[derive(Debug, Clone, PartialEq)]
pub enum SLAStatus {
    OnTrack,   // Plenty of time remaining
    AtRisk,    // Need to prioritize
    Critical,  // Immediate action required
    Violated,  // SLA deadline missed
    Completed, // Job completed on time
}

/// SLA violation alert
#[derive(Debug, Clone)]
pub struct SLAViolationAlert {
    pub job_id: JobId,
    pub deadline: DateTime<Utc>,
    pub violation_time: DateTime<Utc>,
    pub sla_level: SLALevel,
    pub queue_position: Option<usize>,
    pub assigned_worker: Option<String>,
}

/// SLA violation event
#[derive(Debug, Clone)]
pub struct SLAViolationEvent {
    pub job_id: JobId,
    pub deadline: DateTime<Utc>,
    pub violation_time: DateTime<Utc>,
    pub sla_level: SLALevel,
    pub wait_time: Duration,
    pub priority_at_violation: u8,
}

/// SLA statistics
#[derive(Debug, Clone)]
pub struct SLAStats {
    pub total_tracked: u64,
    pub on_time_completions: u64,
    pub sla_violations: u64,
    pub compliance_rate: f64, // Percentage
    pub average_wait_time: Duration,
    pub average_deadline_buffer: Duration,
}

/// SLA metrics snapshot
#[derive(Debug, Clone)]
pub struct SLAMetricsSnapshot {
    pub total_jobs: u64,
    pub at_risk_jobs: u64,
    pub critical_jobs: u64,
    pub violated_jobs: u64,
    pub compliance_rate: f64,
    pub average_time_remaining: Duration,
}

/// Priority adjustment strategy
#[derive(Debug, Clone)]
pub enum PriorityAdjustment {
    None,
    Linear,      // Gradually increase priority as deadline approaches
    Exponential, // Rapid priority increase near deadline
    Immediate,   // Jump to high priority when at risk
}

/// SLA tracker
#[derive(Debug)]
pub struct SLATracker {
    pub jobs: Arc<RwLock<HashMap<JobId, SLAInfo>>>,
    pub violation_events: Arc<RwLock<VecDeque<SLAViolationEvent>>>,
    pub priority_adjustment: PriorityAdjustment,
    pub at_risk_threshold: f64, // Percentage of deadline elapsed (0.0 to 1.0)
    pub critical_threshold: f64, // Percentage of deadline elapsed (0.0 to 1.0)
    pub max_violation_events: usize,
}

impl SLATracker {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            violation_events: Arc::new(RwLock::new(VecDeque::new())),
            priority_adjustment: PriorityAdjustment::Linear,
            at_risk_threshold: 0.7,  // 70% of deadline elapsed
            critical_threshold: 0.9, // 90% of deadline elapsed
            max_violation_events: 1000,
        }
    }

    pub fn with_configuration(
        priority_adjustment: PriorityAdjustment,
        at_risk_threshold: f64,
        critical_threshold: f64,
    ) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            violation_events: Arc::new(RwLock::new(VecDeque::new())),
            priority_adjustment,
            at_risk_threshold,
            critical_threshold,
            max_violation_events: 1000,
        }
    }

    /// Register a job for SLA tracking
    pub async fn register_job(
        &self,
        job_id: JobId,
        sla_level: SLALevel,
        _queue_position: usize,
    ) -> SLAInfo {
        let deadline = match sla_level {
            SLALevel::Critical => Utc::now() + Duration::from_secs(5 * 60),
            SLALevel::High => Utc::now() + Duration::from_secs(15 * 60),
            SLALevel::Medium => Utc::now() + Duration::from_secs(60 * 60),
            SLALevel::Low => Utc::now() + Duration::from_secs(4 * 60 * 60),
            SLALevel::BestEffort => Utc::now() + Duration::from_secs(24 * 60 * 60),
        };

        let sla_info = SLAInfo {
            job_id,
            deadline,
            sla_level: sla_level.clone(),
            priority_boost: 0,
            created_at: Utc::now(),
        };

        info!(
            job_id = %job_id,
            sla_level = ?sla_level,
            deadline = ?deadline,
            "Job registered for SLA tracking"
        );

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id, sla_info.clone());

        sla_info
    }

    /// Update queue position for a job
    pub async fn update_queue_position(&self, job_id: &JobId, position: usize) {
        let mut jobs = self.jobs.write().await;
        if let Some(sla_info) = jobs.get_mut(job_id) {
            // Priority boost based on queue position and SLA level
            sla_info.priority_boost = self.calculate_priority_boost(sla_info, position);
        }
    }

    /// Calculate priority boost based on deadline urgency and queue position
    fn calculate_priority_boost(&self, sla_info: &SLAInfo, _queue_position: usize) -> u8 {
        let now = Utc::now();
        let total_duration = sla_info.deadline - sla_info.created_at;
        let elapsed = now - sla_info.created_at;
        let elapsed_ratio = elapsed.num_seconds() as f64 / total_duration.num_seconds() as f64;

        match self.priority_adjustment {
            PriorityAdjustment::None => 0,
            PriorityAdjustment::Linear => {
                if elapsed_ratio > self.at_risk_threshold {
                    ((elapsed_ratio - self.at_risk_threshold) * 10.0) as u8
                } else {
                    0
                }
            }
            PriorityAdjustment::Exponential => {
                if elapsed_ratio > self.at_risk_threshold {
                    let excess = elapsed_ratio - self.at_risk_threshold;
                    (excess * excess * 20.0) as u8
                } else {
                    0
                }
            }
            PriorityAdjustment::Immediate => {
                if elapsed_ratio > self.at_risk_threshold {
                    5
                } else {
                    0
                }
            }
        }
    }

    /// Get SLA status for a job
    pub async fn get_sla_status(&self, job_id: &JobId) -> Option<(SLAStatus, Duration)> {
        let jobs = self.jobs.read().await;
        let sla_info = jobs.get(job_id)?;

        let now = Utc::now();

        if now > sla_info.deadline {
            return Some((SLAStatus::Violated, Duration::from_secs(0)));
        }

        let time_remaining = sla_info.deadline - now;
        let total_duration = sla_info.deadline - sla_info.created_at;
        let elapsed_ratio =
            (now - sla_info.created_at).num_seconds() as f64 / total_duration.num_seconds() as f64;

        let status = if elapsed_ratio > self.critical_threshold {
            SLAStatus::Critical
        } else if elapsed_ratio > self.at_risk_threshold {
            SLAStatus::AtRisk
        } else {
            SLAStatus::OnTrack
        };

        Some((status, time_remaining.to_std().unwrap_or_default()))
    }

    /// Check for SLA violations
    pub async fn check_violations(&self) -> Vec<SLAViolationAlert> {
        let now = Utc::now();
        let mut jobs = self.jobs.write().await;
        let mut alerts = Vec::new();

        let mut violated_jobs = Vec::new();

        for (job_id, sla_info) in jobs.iter_mut() {
            if now > sla_info.deadline {
                let violation_time = now;
                violated_jobs.push(*job_id);

                let alert = SLAViolationAlert {
                    job_id: *job_id,
                    deadline: sla_info.deadline,
                    violation_time,
                    sla_level: sla_info.sla_level.clone(),
                    queue_position: None, // Would need to be provided externally
                    assigned_worker: None,
                };
                alerts.push(alert);
            }
        }

        // Record violation events
        for job_id in violated_jobs {
            if let Some(sla_info) = jobs.remove(&job_id) {
                let event = SLAViolationEvent {
                    job_id,
                    deadline: sla_info.deadline,
                    violation_time: now,
                    sla_level: sla_info.sla_level,
                    wait_time: (now - sla_info.created_at).to_std().unwrap_or_default(),
                    priority_at_violation: sla_info.priority_boost,
                };

                self.record_violation_event(event).await;
            }
        }

        if !alerts.is_empty() {
            warn!(count = alerts.len(), "SLA violations detected");
        }

        alerts
    }

    /// Record a violation event
    async fn record_violation_event(&self, event: SLAViolationEvent) {
        let mut events = self.violation_events.write().await;
        events.push_back(event);

        // Keep only the most recent events
        while events.len() > self.max_violation_events {
            events.pop_front();
        }
    }

    /// Mark job as completed
    pub async fn mark_completed(&self, job_id: &JobId) -> Option<SLAStatus> {
        let mut jobs = self.jobs.write().await;
        let sla_info = jobs.remove(job_id)?;

        let now = Utc::now();
        let status = if now <= sla_info.deadline {
            SLAStatus::Completed
        } else {
            SLAStatus::Violated
        };

        info!(
            job_id = %job_id,
            sla_level = ?sla_info.sla_level,
            status = ?status,
            "Job completed or violated"
        );

        Some(status)
    }

    /// Get SLA statistics
    pub async fn get_stats(&self) -> SLAStats {
        let jobs = self.jobs.read().await;
        let events = self.violation_events.read().await;

        let total_tracked = jobs.len() as u64 + events.len() as u64;
        let on_time_completions = events
            .iter()
            .filter(|e| e.violation_time <= e.deadline)
            .count() as u64;
        let sla_violations = events.len() as u64;

        let compliance_rate = if total_tracked > 0 {
            (on_time_completions as f64 / total_tracked as f64) * 100.0
        } else {
            100.0
        };

        let avg_wait = if !events.is_empty() {
            let total_wait: Duration = events
                .iter()
                .map(|e| e.wait_time)
                .fold(Duration::from_secs(0), |acc, d| acc + d);
            Duration::from_nanos(total_wait.as_nanos() as u64 / events.len() as u64)
        } else {
            Duration::from_secs(0)
        };

        let avg_buffer = if !jobs.is_empty() {
            let now = Utc::now();
            let total_buffer: Duration = jobs
                .values()
                .map(|j| (j.deadline - now).to_std().unwrap_or_default())
                .fold(Duration::from_secs(0), |acc, d| acc + d);
            Duration::from_nanos(total_buffer.as_nanos() as u64 / jobs.len() as u64)
        } else {
            Duration::from_secs(0)
        };

        SLAStats {
            total_tracked,
            on_time_completions,
            sla_violations,
            compliance_rate,
            average_wait_time: avg_wait,
            average_deadline_buffer: avg_buffer,
        }
    }

    /// Get metrics snapshot
    pub async fn get_metrics(&self) -> SLAMetricsSnapshot {
        let jobs = self.jobs.read().await;

        let now = Utc::now();
        let mut at_risk_count = 0;
        let mut critical_count = 0;
        let violated_count = 0;
        let mut total_time_remaining = Duration::from_secs(0);
        let mut jobs_with_time = 0;

        for sla_info in jobs.values() {
            let elapsed_ratio = (now - sla_info.created_at).num_seconds() as f64
                / (sla_info.deadline - sla_info.created_at).num_seconds() as f64;

            if elapsed_ratio > self.critical_threshold {
                critical_count += 1;
            } else if elapsed_ratio > self.at_risk_threshold {
                at_risk_count += 1;
            }

            let time_remaining = (sla_info.deadline - now).to_std().unwrap_or_default();
            total_time_remaining += time_remaining;
            jobs_with_time += 1;
        }

        // Calculate compliance rate
        let stats = self.get_stats().await;
        let avg_time_remaining = if jobs_with_time > 0 {
            Duration::from_nanos(total_time_remaining.as_nanos() as u64 / jobs_with_time as u64)
        } else {
            Duration::from_secs(0)
        };

        SLAMetricsSnapshot {
            total_jobs: jobs.len() as u64,
            at_risk_jobs: at_risk_count,
            critical_jobs: critical_count,
            violated_jobs: violated_count,
            compliance_rate: stats.compliance_rate,
            average_time_remaining: avg_time_remaining,
        }
    }

    /// Get all at-risk jobs
    pub async fn get_at_risk_jobs(&self) -> Vec<JobId> {
        let jobs = self.jobs.read().await;
        let now = Utc::now();

        jobs.iter()
            .filter(|(_, sla_info)| {
                let elapsed_ratio = (now - sla_info.created_at).num_seconds() as f64
                    / (sla_info.deadline - sla_info.created_at).num_seconds() as f64;
                elapsed_ratio > self.at_risk_threshold && elapsed_ratio <= self.critical_threshold
            })
            .map(|(job_id, _)| *job_id)
            .collect()
    }

    /// Get all critical jobs
    pub async fn get_critical_jobs(&self) -> Vec<JobId> {
        let jobs = self.jobs.read().await;
        let now = Utc::now();

        jobs.iter()
            .filter(|(_, sla_info)| {
                let elapsed_ratio = (now - sla_info.created_at).num_seconds() as f64
                    / (sla_info.deadline - sla_info.created_at).num_seconds() as f64;
                elapsed_ratio > self.critical_threshold
            })
            .map(|(job_id, _)| *job_id)
            .collect()
    }
}

impl Default for SLATracker {
    fn default() -> Self {
        Self::new()
    }
}
