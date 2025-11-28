//! Cost Tracking Module
//!
//! This module provides cost tracking and reporting for dynamic resource pools
//! with per-worker-hour calculation, job cost attribution, and billing integration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use hodei_core::{DomainError, Result};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Cost per hour for different worker types
#[derive(Debug, Clone)]
pub struct WorkerCost {
    pub worker_type: String,
    pub cost_per_hour_cents: u64, // Cost in cents for precision
    pub cpu_cost_per_hour_cents: u64,
    pub memory_cost_per_hour_cents: u64,
    pub storage_cost_per_hour_cents: u64,
}

impl WorkerCost {
    pub fn new(worker_type: String, cost_per_hour_cents: u64) -> Self {
        Self {
            worker_type,
            cost_per_hour_cents,
            cpu_cost_per_hour_cents: 0,
            memory_cost_per_hour_cents: 0,
            storage_cost_per_hour_cents: 0,
        }
    }

    pub fn with_resource_costs(
        worker_type: String,
        cost_per_hour_cents: u64,
        cpu_cost_per_hour_cents: u64,
        memory_cost_per_hour_cents: u64,
        storage_cost_per_hour_cents: u64,
    ) -> Self {
        Self {
            worker_type,
            cost_per_hour_cents,
            cpu_cost_per_hour_cents,
            memory_cost_per_hour_cents,
            storage_cost_per_hour_cents,
        }
    }

    /// Calculate total hourly cost
    pub fn total_cost_per_hour_cents(&self) -> u64 {
        self.cost_per_hour_cents
            + self.cpu_cost_per_hour_cents
            + self.memory_cost_per_hour_cents
            + self.storage_cost_per_hour_cents
    }

    /// Calculate cost for a specific duration
    pub fn cost_for_duration_cents(&self, duration: Duration) -> u64 {
        let hours = duration.as_secs_f64() / 3600.0;
        (self.total_cost_per_hour_cents() as f64 * hours).round() as u64
    }
}

/// Job cost attribution
#[derive(Debug, Clone)]
pub struct JobCost {
    pub job_id: String,
    pub tenant_id: String,
    pub pool_id: String,
    pub worker_id: String,
    pub worker_type: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub cpu_cores_used: u32,
    pub memory_gb_used: u64,
    pub duration_seconds: Option<u64>,
    pub total_cost_cents: u64,
}

impl JobCost {
    pub fn new(
        job_id: String,
        tenant_id: String,
        pool_id: String,
        worker_id: String,
        worker_type: String,
        start_time: DateTime<Utc>,
        cpu_cores_used: u32,
        memory_gb_used: u64,
    ) -> Self {
        Self {
            job_id,
            tenant_id,
            pool_id,
            worker_id,
            worker_type,
            start_time,
            end_time: None,
            cpu_cores_used,
            memory_gb_used,
            duration_seconds: None,
            total_cost_cents: 0,
        }
    }

    /// Calculate final cost when job completes
    pub fn calculate_final_cost(&mut self, worker_cost: &WorkerCost) {
        if let Some(end_time) = self.end_time {
            let duration = end_time - self.start_time;
            self.duration_seconds = Some(duration.num_seconds() as u64);
            self.total_cost_cents =
                worker_cost.cost_for_duration_cents(duration.to_std().unwrap_or_default());
        }
    }

    /// Get job duration in seconds
    pub fn duration(&self) -> Option<Duration> {
        self.end_time
            .and_then(|end| (end - self.start_time).to_std().ok())
    }
}

/// Cost reporting period
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CostReportingPeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

/// Cost summary by period
#[derive(Debug, Clone)]
pub struct CostSummary {
    pub period: CostReportingPeriod,
    pub total_cost_cents: u64,
    pub total_jobs: u64,
    pub total_worker_hours: f64,
    pub average_cost_per_job_cents: u64,
    pub cost_by_tenant: HashMap<String, u64>,
    pub cost_by_pool: HashMap<String, u64>,
    pub cost_by_worker_type: HashMap<String, u64>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl CostSummary {
    pub fn new(period: CostReportingPeriod) -> Self {
        let now = Utc::now();
        Self {
            period,
            total_cost_cents: 0,
            total_jobs: 0,
            total_worker_hours: 0.0,
            average_cost_per_job_cents: 0,
            cost_by_tenant: HashMap::new(),
            cost_by_pool: HashMap::new(),
            cost_by_worker_type: HashMap::new(),
            start_time: now,
            end_time: now,
        }
    }

    pub fn add_job_cost(&mut self, job_cost: &JobCost) {
        self.total_cost_cents += job_cost.total_cost_cents;
        self.total_jobs += 1;

        // Track by tenant
        *self
            .cost_by_tenant
            .entry(job_cost.tenant_id.clone())
            .or_insert(0) += job_cost.total_cost_cents;

        // Track by pool
        *self
            .cost_by_pool
            .entry(job_cost.pool_id.clone())
            .or_insert(0) += job_cost.total_cost_cents;

        // Track by worker type
        *self
            .cost_by_worker_type
            .entry(job_cost.worker_type.clone())
            .or_insert(0) += job_cost.total_cost_cents;

        // Update average
        self.average_cost_per_job_cents = self.total_cost_cents / self.total_jobs;
    }

    /// Format cost in dollars for display
    pub fn total_cost_dollars(&self) -> f64 {
        self.total_cost_cents as f64 / 100.0
    }

    /// Format average cost per job in dollars
    pub fn average_cost_per_job_dollars(&self) -> f64 {
        self.average_cost_per_job_cents as f64 / 100.0
    }
}

/// Cost tracking service
#[derive(Clone)]
pub struct CostTrackingService {
    worker_costs: Arc<RwLock<HashMap<String, WorkerCost>>>,
    active_jobs: Arc<RwLock<HashMap<String, JobCost>>>,
    completed_jobs: Arc<RwLock<Vec<JobCost>>>,
    cost_alerts: Arc<RwLock<CostAlerts>>,
}

impl CostTrackingService {
    pub fn new() -> Self {
        Self {
            worker_costs: Arc::new(RwLock::new(HashMap::new())),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            completed_jobs: Arc::new(RwLock::new(Vec::new())),
            cost_alerts: Arc::new(RwLock::new(CostAlerts::new())),
        }
    }

    /// Register a worker cost configuration
    pub async fn register_worker_cost(&self, worker_cost: WorkerCost) {
        let worker_type = worker_cost.worker_type.clone();
        let cost_per_hour = worker_cost.total_cost_per_hour_cents();

        let mut costs = self.worker_costs.write().await;
        costs.insert(worker_type.clone(), worker_cost);

        info!(
            worker_type = worker_type,
            cost_per_hour = cost_per_hour,
            "Worker cost registered"
        );
    }

    /// Start tracking a job
    pub async fn start_job_tracking(&self, job_cost: JobCost) -> Result<()> {
        let job_id = job_cost.job_id.clone();

        let mut active_jobs = self.active_jobs.write().await;

        if active_jobs.contains_key(&job_id) {
            return Err(CostTrackingError::JobAlreadyTracked(job_id).into());
        }

        active_jobs.insert(job_id.clone(), job_cost);
        info!(job_id = job_id, "Started cost tracking for job");
        Ok(())
    }

    /// Complete a job and calculate final cost
    pub async fn complete_job_tracking(&self, job_id: &str) -> Result<JobCost> {
        let mut active_jobs = self.active_jobs.write().await;
        let mut completed_jobs = self.completed_jobs.write().await;

        let job_cost = active_jobs
            .remove(job_id)
            .ok_or_else(|| CostTrackingError::JobNotFound(job_id.to_string()))?;

        let mut job_cost = job_cost;
        let worker_type = job_cost.worker_type.clone();

        // Calculate final cost using worker type
        let worker_cost = {
            let costs = self.worker_costs.read().await;
            match costs.get(&worker_type) {
                Some(cost) => cost.clone(),
                None => return Err(CostTrackingError::WorkerTypeNotFound(worker_type).into()),
            }
        };

        job_cost.end_time = Some(Utc::now());
        job_cost.calculate_final_cost(&worker_cost);

        completed_jobs.push(job_cost.clone());

        // Check for cost alerts
        let mut alerts = self.cost_alerts.write().await;
        alerts.check_job_cost(&job_cost);

        info!(
            job_id = job_cost.job_id,
            cost_cents = job_cost.total_cost_cents,
            "Completed job cost tracking"
        );
        Ok(job_cost)
    }

    /// Get cost for a specific job
    pub async fn get_job_cost(&self, job_id: &str) -> Option<JobCost> {
        let active_jobs = self.active_jobs.read().await;
        let completed_jobs = self.completed_jobs.read().await;

        if let Some(job) = active_jobs.get(job_id) {
            Some(job.clone())
        } else {
            completed_jobs.iter().find(|j| j.job_id == job_id).cloned()
        }
    }

    /// Generate cost summary for a period
    pub async fn generate_cost_summary(&self, period: CostReportingPeriod) -> CostSummary {
        let mut summary = CostSummary::new(period.clone());

        let completed_jobs = self.completed_jobs.read().await;

        // Filter jobs by period
        let now = Utc::now();
        let (period_start, period_end) = match &period {
            CostReportingPeriod::Hourly => {
                let start = now - chrono::Duration::hours(1);
                (start, now)
            }
            CostReportingPeriod::Daily => {
                let start = now - chrono::Duration::days(1);
                (start, now)
            }
            CostReportingPeriod::Weekly => {
                let start = now - chrono::Duration::days(7);
                (start, now)
            }
            CostReportingPeriod::Monthly => {
                let start = now - chrono::Duration::days(30);
                (start, now)
            }
            CostReportingPeriod::Custom { start, end } => (*start, *end),
        };

        summary.start_time = period_start;
        summary.end_time = period_end;

        // Aggregate costs
        for job in completed_jobs.iter() {
            if let Some(end_time) = job.end_time
                && end_time >= period_start
                && end_time <= period_end
            {
                summary.add_job_cost(job);

                // Calculate total worker hours
                if let Some(duration_seconds) = job.duration_seconds {
                    summary.total_worker_hours += duration_seconds as f64 / 3600.0;
                }
            }
        }

        summary
    }

    /// Get total cost across all completed jobs
    pub async fn get_total_cost(&self) -> u64 {
        let completed_jobs = self.completed_jobs.read().await;
        completed_jobs.iter().map(|j| j.total_cost_cents).sum()
    }

    /// Get cost by tenant
    pub async fn get_cost_by_tenant(&self) -> HashMap<String, u64> {
        let completed_jobs = self.completed_jobs.read().await;
        let mut cost_by_tenant = HashMap::new();

        for job in completed_jobs.iter() {
            *cost_by_tenant.entry(job.tenant_id.clone()).or_insert(0) += job.total_cost_cents;
        }

        cost_by_tenant
    }

    /// Get active job count
    pub async fn get_active_job_count(&self) -> usize {
        let active_jobs = self.active_jobs.read().await;
        active_jobs.len()
    }

    /// Get completed job count
    pub async fn get_completed_job_count(&self) -> usize {
        let completed_jobs = self.completed_jobs.read().await;
        completed_jobs.len()
    }

    /// Get current cost alerts
    pub async fn get_cost_alerts(&self) -> CostAlerts {
        let alerts = self.cost_alerts.read().await;
        alerts.clone()
    }
}

impl Default for CostTrackingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost alert configuration and status
#[derive(Debug, Clone)]
pub struct CostAlerts {
    pub max_cost_per_job_cents: Option<u64>,
    pub max_cost_per_hour_cents: Option<u64>,
    pub max_cost_per_day_cents: Option<u64>,
    pub active_alerts: Vec<CostAlert>,
}

impl Default for CostAlerts {
    fn default() -> Self {
        Self::new()
    }
}

impl CostAlerts {
    pub fn new() -> Self {
        Self {
            max_cost_per_job_cents: None,
            max_cost_per_hour_cents: None,
            max_cost_per_day_cents: None,
            active_alerts: Vec::new(),
        }
    }

    pub fn set_max_cost_per_job(&mut self, max_cost_cents: u64) {
        self.max_cost_per_job_cents = Some(max_cost_cents);
    }

    pub fn set_max_cost_per_hour(&mut self, max_cost_cents: u64) {
        self.max_cost_per_hour_cents = Some(max_cost_cents);
    }

    pub fn set_max_cost_per_day(&mut self, max_cost_cents: u64) {
        self.max_cost_per_day_cents = Some(max_cost_cents);
    }

    fn check_job_cost(&mut self, job_cost: &JobCost) {
        // Check per-job cost
        if let Some(max_cost) = self.max_cost_per_job_cents
            && job_cost.total_cost_cents > max_cost
        {
            self.active_alerts.push(CostAlert {
                alert_type: CostAlertType::JobCostExceeded,
                job_id: job_cost.job_id.clone(),
                threshold_cents: max_cost,
                actual_cost_cents: job_cost.total_cost_cents,
                timestamp: Utc::now(),
                message: format!(
                    "Job {} cost ${:.2} exceeded threshold ${:.2}",
                    job_cost.job_id,
                    job_cost.total_cost_cents as f64 / 100.0,
                    max_cost as f64 / 100.0
                ),
            });
        }
    }
}

/// Cost alert
#[derive(Debug, Clone)]
pub struct CostAlert {
    pub alert_type: CostAlertType,
    pub job_id: String,
    pub threshold_cents: u64,
    pub actual_cost_cents: u64,
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

/// Cost alert types
#[derive(Debug, Clone)]
pub enum CostAlertType {
    JobCostExceeded,
    DailyBudgetExceeded,
    HourlyBudgetExceeded,
}

/// Errors
#[derive(Error, Debug)]
pub enum CostTrackingError {
    #[error("Job already tracked: {0}")]
    JobAlreadyTracked(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Worker type not found: {0}")]
    WorkerTypeNotFound(String),

    #[error("Invalid cost configuration: {0}")]
    InvalidCostConfiguration(String),
}

// Convert CostTrackingError to DomainError
impl From<CostTrackingError> for hodei_core::DomainError {
    fn from(err: CostTrackingError) -> Self {
        hodei_core::DomainError::Other(err.to_string())
    }
}
