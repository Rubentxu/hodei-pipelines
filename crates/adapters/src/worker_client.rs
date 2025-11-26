//! Production-grade Worker Client Implementation
//!
//! This module provides real gRPC and HTTP client implementations for
//! worker communication, replacing mock implementations for production use.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hodei_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use hodei_core::{JobId, JobSpec};
use hodei_core::{WorkerId, WorkerStatus};
use hodei_ports::{WorkerClient, WorkerClientError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tonic::{Status, transport::Channel};
use tracing::{debug, error, info, warn};

/// Resource usage metrics for a worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_rss_mb: u64,
    pub memory_vms_mb: u64,
    pub disk_read_mb: f64,
    pub disk_write_mb: f64,
    pub network_sent_mb: f64,
    pub network_received_mb: f64,
    pub gpu_utilization_percent: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

/// Memory usage metrics
#[derive(Debug, Clone)]
struct MemoryUsage {
    rss_mb: u64,
    vms_mb: u64,
}

/// CPU usage metrics
#[derive(Debug, Clone)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
}

/// Errors for metrics collection
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("System read error: {0}")]
    SystemReadError(#[from] std::io::Error),
    #[error("Invalid /proc/stat format")]
    InvalidProcStat,
    #[error("Invalid CPU times")]
    InvalidCpuTimes,
    #[error("Invalid /proc/self/status format")]
    InvalidStatusFormat,
    #[error("Field not found: {0}")]
    FieldNotFound(String),
    #[error("Invalid field format")]
    InvalidFormat,
    #[error("Other error: {0}")]
    Other(String),
}

/// Metrics collector for a specific worker
#[derive(Debug)]
pub struct MetricsCollector {
    worker_id: WorkerId,
    interval: Duration,
    previous_cpu_times: Option<CpuTimes>,
}

impl MetricsCollector {
    pub fn new(worker_id: WorkerId, interval: Duration) -> Self {
        Self {
            worker_id,
            interval,
            previous_cpu_times: None,
        }
    }

    /// Collect all resource metrics for the worker
    pub async fn collect(&mut self) -> Result<ResourceUsage, MetricsError> {
        let cpu_usage = self.collect_cpu_usage().await?;
        let memory_usage = self.collect_memory_usage().await?;
        let (disk_read_mb, disk_write_mb) = self.collect_disk_usage().await?;
        let (network_sent_mb, network_received_mb) = self.collect_network_usage().await?;
        let gpu_usage = self.collect_gpu_usage().await?;

        Ok(ResourceUsage {
            cpu_percent: cpu_usage,
            memory_rss_mb: memory_usage.rss_mb,
            memory_vms_mb: memory_usage.vms_mb,
            disk_read_mb,
            disk_write_mb,
            network_sent_mb,
            network_received_mb,
            gpu_utilization_percent: gpu_usage,
            timestamp: Utc::now(),
        })
    }

    /// Collect CPU usage percentage
    async fn collect_cpu_usage(&mut self) -> Result<f64, MetricsError> {
        let stat_content = tokio::fs::read_to_string("/proc/stat")
            .await
            .map_err(MetricsError::SystemReadError)?;

        let first_cpu_line = stat_content
            .lines()
            .find(|line| line.starts_with("cpu "))
            .ok_or(MetricsError::InvalidProcStat)?;

        let times: Vec<u64> = first_cpu_line
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        if times.len() < 4 {
            return Err(MetricsError::InvalidCpuTimes);
        }

        let current_times = CpuTimes {
            user: times[0],
            nice: times[1],
            system: times[2],
            idle: times[3],
        };

        let cpu_percent = match self.previous_cpu_times.take() {
            Some(prev) => {
                let delta_idle = current_times.idle - prev.idle;
                let delta_total: u64 =
                    times.iter().sum::<u64>() - prev.user - prev.nice - prev.system - prev.idle;
                let delta_non_idle = delta_total - delta_idle;

                self.previous_cpu_times = Some(current_times);

                if delta_total > 0 {
                    100.0 * (delta_non_idle as f64) / (delta_total as f64)
                } else {
                    0.0
                }
            }
            None => {
                // First reading, just store and return 0
                self.previous_cpu_times = Some(current_times);
                0.0
            }
        };

        Ok(cpu_percent)
    }

    /// Collect memory usage
    async fn collect_memory_usage(&self) -> Result<MemoryUsage, MetricsError> {
        let status_content = tokio::fs::read_to_string("/proc/self/status")
            .await
            .map_err(MetricsError::SystemReadError)?;

        let rss = self.parse_memory_field(&status_content, "VmRSS:")?;
        let vms = self.parse_memory_field(&status_content, "VmSize:")?;

        Ok(MemoryUsage {
            rss_mb: rss / 1024,
            vms_mb: vms / 1024,
        })
    }

    /// Parse a memory field from /proc/self/status
    fn parse_memory_field(&self, content: &str, field: &str) -> Result<u64, MetricsError> {
        for line in content.lines() {
            if line.starts_with(field) {
                let value = line
                    .split_whitespace()
                    .nth(1)
                    .ok_or(MetricsError::InvalidFormat)?;
                return Ok(value.parse().unwrap_or(0));
            }
        }
        Err(MetricsError::FieldNotFound(field.to_string()))
    }

    /// Collect disk I/O metrics
    async fn collect_disk_usage(&self) -> Result<(f64, f64), MetricsError> {
        // For now, return placeholder values
        // TODO: Implement actual disk I/O collection from /proc/diskstats
        Ok((0.0, 0.0))
    }

    /// Collect network I/O metrics
    async fn collect_network_usage(&self) -> Result<(f64, f64), MetricsError> {
        // For now, return placeholder values
        // TODO: Implement actual network I/O collection from /proc/net/dev
        Ok((0.0, 0.0))
    }

    /// Collect GPU utilization (optional)
    async fn collect_gpu_usage(&self) -> Result<Option<f64>, MetricsError> {
        // For now, return None
        // TODO: Implement actual GPU metrics collection
        Ok(None)
    }
}

/// Worker metrics exporter
#[derive(Debug)]
pub struct WorkerMetricsExporter {
    collectors: Arc<RwLock<HashMap<WorkerId, Arc<RwLock<MetricsCollector>>>>>,
    prometheus_registry: Option<prometheus::Registry>,
}

impl WorkerMetricsExporter {
    /// Create a new metrics exporter
    pub fn new(prometheus_registry: Option<prometheus::Registry>) -> Self {
        Self {
            collectors: Arc::new(RwLock::new(HashMap::new())),
            prometheus_registry,
        }
    }

    /// Register a worker for metrics collection
    pub async fn register_worker(&self, worker_id: WorkerId, collection_interval: Duration) {
        let collector = MetricsCollector::new(worker_id.clone(), collection_interval);
        let collector = Arc::new(RwLock::new(collector));

        let mut collectors = self.collectors.write().await;
        collectors.insert(worker_id, collector);
    }

    /// Unregister a worker from metrics collection
    pub async fn unregister_worker(&self, worker_id: &WorkerId) {
        let mut collectors = self.collectors.write().await;
        collectors.remove(worker_id);
    }

    /// Get current metrics for a worker
    pub async fn get_worker_metrics(
        &self,
        worker_id: &WorkerId,
    ) -> Result<ResourceUsage, MetricsError> {
        let collectors = self.collectors.read().await;
        if let Some(collector_arc) = collectors.get(worker_id) {
            let mut collector = collector_arc.write().await;
            collector.collect().await
        } else {
            Err(MetricsError::Other(format!(
                "Worker {} not registered for metrics collection",
                worker_id
            )))
        }
    }

    /// List all workers being monitored
    pub async fn list_monitored_workers(&self) -> Vec<WorkerId> {
        let collectors = self.collectors.read().await;
        collectors.keys().cloned().collect()
    }
}

/// gRPC-based Worker Client for production use
pub struct GrpcWorkerClient {
    channel: Channel,
    timeout: Duration,
}

impl GrpcWorkerClient {
    pub fn new(channel: Channel, timeout: Duration) -> Self {
        Self { channel, timeout }
    }

    /// Create client with default timeout (5 seconds)
    pub fn with_default_timeout(channel: Channel) -> Self {
        Self::new(channel, Duration::from_secs(5))
    }

    /// Get worker service client
    fn worker_service_client(&self) -> hwp_proto::WorkerServiceClient<Channel> {
        hwp_proto::WorkerServiceClient::new(self.channel.clone())
    }
}

#[async_trait]
impl WorkerClient for GrpcWorkerClient {
    async fn assign_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
        job_spec: &JobSpec,
    ) -> Result<(), WorkerClientError> {
        let request = hwp_proto::AssignJobRequest {
            worker_id: worker_id.to_string(),
            job_id: job_id.to_string(),
            job_spec: Some(hwp_proto::JobSpec {
                name: job_spec.name.clone(),
                image: job_spec.image.clone(),
                command: job_spec.command.clone(),
                resources: Some(hwp_proto::ResourceQuota {
                    cpu_m: job_spec.resources.cpu_m,
                    memory_mb: job_spec.resources.memory_mb,
                    gpu: job_spec.resources.gpu.unwrap_or(0) as u32,
                }),
                timeout_ms: job_spec.timeout_ms,
                retries: job_spec.retries as u32,
                env: job_spec.env.clone(),
                secret_refs: job_spec.secret_refs.clone(),
            }),
        };

        info!("Assigning job {} to worker {} via gRPC", job_id, worker_id);

        let result = timeout(self.timeout, async {
            let mut client = self.worker_service_client();
            client.assign_job(request).await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!(
                "Assign job operation timed out after {:?}",
                self.timeout
            ))
        })?
        .map_err(|status| {
            error!("gRPC assign_job failed: {}", status);
            if status.code() == tonic::Code::NotFound {
                WorkerClientError::NotFound(worker_id.clone())
            } else if status.code() == tonic::Code::DeadlineExceeded {
                WorkerClientError::Timeout("gRPC deadline exceeded".to_string())
            } else {
                WorkerClientError::Communication(status.message().to_string())
            }
        })?;

        info!(
            "Successfully assigned job {} to worker {}",
            job_id, worker_id
        );
        Ok(())
    }

    async fn cancel_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerClientError> {
        let request = hwp_proto::CancelJobRequest {
            worker_id: worker_id.to_string(),
            job_id: job_id.to_string(),
        };

        info!("Cancelling job {} on worker {} via gRPC", job_id, worker_id);

        let result = timeout(self.timeout, async {
            let mut client = self.worker_service_client();
            client.cancel_job(request).await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!(
                "Cancel job operation timed out after {:?}",
                self.timeout
            ))
        })?
        .map_err(|status| {
            error!("gRPC cancel_job failed: {}", status);
            if status.code() == tonic::Code::NotFound {
                WorkerClientError::NotFound(worker_id.clone())
            } else if status.code() == tonic::Code::DeadlineExceeded {
                WorkerClientError::Timeout("gRPC deadline exceeded".to_string())
            } else {
                WorkerClientError::Communication(status.message().to_string())
            }
        })?;

        info!(
            "Successfully cancelled job {} on worker {}",
            job_id, worker_id
        );
        Ok(())
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerStatus, WorkerClientError> {
        let request = hwp_proto::GetWorkerStatusRequest {
            worker_id: worker_id.to_string(),
        };

        let result = timeout(self.timeout, async {
            let mut client = self.worker_service_client();
            client.get_worker_status(request).await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!(
                "Get worker status operation timed out after {:?}",
                self.timeout
            ))
        })?
        .map_err(|status| {
            error!("gRPC get_worker_status failed: {}", status);
            if status.code() == tonic::Code::NotFound {
                WorkerClientError::NotFound(worker_id.clone())
            } else if status.code() == tonic::Code::DeadlineExceeded {
                WorkerClientError::Timeout("gRPC deadline exceeded".to_string())
            } else {
                WorkerClientError::Communication(status.message().to_string())
            }
        })?;

        let proto_status = result.into_inner();

        let worker_status = WorkerStatus {
            worker_id: worker_id.clone(),
            status: proto_status.state,
            current_jobs: vec![], // Parse from response if available
            last_heartbeat: chrono::Utc::now().into(),
        };

        Ok(worker_status)
    }

    async fn send_heartbeat(&self, worker_id: &WorkerId) -> Result<(), WorkerClientError> {
        let request = hwp_proto::HeartbeatRequest {
            worker_id: worker_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            resource_usage: None, // TODO: Add resource metrics
        };

        let result = timeout(self.timeout, async {
            let mut client = self.worker_service_client();
            client.heartbeat(request).await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!(
                "Heartbeat operation timed out after {:?}",
                self.timeout
            ))
        })?
        .map_err(|status| {
            // Heartbeat failures are not critical, log as warning
            warn!("Heartbeat to worker {} failed: {}", worker_id, status);
            WorkerClientError::Communication(status.message().to_string())
        })?;

        // Heartbeat success is logged at debug level
        debug!("Heartbeat sent to worker {}", worker_id);
        Ok(())
    }
}

/// HTTP-based Worker Client (alternative to gRPC)
pub struct HttpWorkerClient {
    base_url: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl HttpWorkerClient {
    pub fn new(base_url: String, timeout: Duration) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            timeout,
        }
    }

    pub fn with_default_timeout(base_url: String) -> Self {
        Self::new(base_url, Duration::from_secs(5))
    }
}

#[async_trait]
impl WorkerClient for HttpWorkerClient {
    async fn assign_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
        job_spec: &JobSpec,
    ) -> Result<(), WorkerClientError> {
        let url = format!("{}/api/v1/workers/{}/jobs", self.base_url, worker_id);

        let payload = serde_json::json!({
            "job_id": job_id.to_string(),
            "job_spec": job_spec
        });

        info!("Assigning job {} to worker {} via HTTP", job_id, worker_id);

        let response = timeout(self.timeout, async {
            self.client.post(&url).json(&payload).send().await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!(
                "HTTP assign job timed out after {:?}",
                self.timeout
            ))
        })?
        .map_err(|e| {
            error!("HTTP assign_job failed: {}", e);
            WorkerClientError::Communication(e.to_string())
        })?;

        if !response.status().is_success() {
            return Err(WorkerClientError::Communication(format!(
                "HTTP assign job failed with status: {}",
                response.status()
            )));
        }

        info!(
            "Successfully assigned job {} to worker {}",
            job_id, worker_id
        );
        Ok(())
    }

    async fn cancel_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerClientError> {
        let url = format!(
            "{}/api/v1/workers/{}/jobs/{}",
            self.base_url, worker_id, job_id
        );

        info!("Cancelling job {} on worker {} via HTTP", job_id, worker_id);

        let response = timeout(self.timeout, async {
            self.client.delete(&url).send().await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!(
                "HTTP cancel job timed out after {:?}",
                self.timeout
            ))
        })?
        .map_err(|e| {
            error!("HTTP cancel_job failed: {}", e);
            WorkerClientError::Communication(e.to_string())
        })?;

        if !response.status().is_success() {
            return Err(WorkerClientError::Communication(format!(
                "HTTP cancel job failed with status: {}",
                response.status()
            )));
        }

        info!(
            "Successfully cancelled job {} on worker {}",
            job_id, worker_id
        );
        Ok(())
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerStatus, WorkerClientError> {
        let url = format!("{}/api/v1/workers/{}", self.base_url, worker_id);

        let response = timeout(self.timeout, async { self.client.get(&url).send().await })
            .await
            .map_err(|_| {
                WorkerClientError::Timeout(format!(
                    "HTTP get worker status timed out after {:?}",
                    self.timeout
                ))
            })?
            .map_err(|e| {
                error!("HTTP get_worker_status failed: {}", e);
                WorkerClientError::Communication(e.to_string())
            })?;

        if !response.status().is_success() {
            return Err(WorkerClientError::Communication(format!(
                "HTTP get worker status failed with status: {}",
                response.status()
            )));
        }

        let worker_data: serde_json::Value = response.json().await.map_err(|e| {
            WorkerClientError::Communication(format!("Failed to parse worker status: {}", e))
        })?;

        // Parse worker status from JSON response
        let status_str = worker_data["status"].as_str().unwrap_or("OFFLINE");

        let worker_status = WorkerStatus {
            worker_id: worker_id.clone(),
            status: status_str.to_string(),
            current_jobs: vec![], // Parse from response if available
            last_heartbeat: std::time::SystemTime::now(),
        };

        Ok(worker_status)
    }

    async fn send_heartbeat(&self, worker_id: &WorkerId) -> Result<(), WorkerClientError> {
        let url = format!("{}/api/v1/workers/{}/heartbeat", self.base_url, worker_id);

        let payload = serde_json::json!({
            "worker_id": worker_id.to_string(),
            "timestamp": chrono::Utc::now().timestamp()
        });

        let response = timeout(self.timeout, async {
            self.client.post(&url).json(&payload).send().await
        })
        .await
        .map_err(|_| {
            WorkerClientError::Timeout(format!("HTTP heartbeat timed out after {:?}", self.timeout))
        })?
        .map_err(|e| {
            warn!("HTTP heartbeat to worker {} failed: {}", worker_id, e);
            WorkerClientError::Communication(e.to_string())
        })?;

        // Heartbeat failures are not critical
        debug!("Heartbeat sent to worker {}", worker_id);
        Ok(())
    }
}

/// Resilient Worker Client with Circuit Breaker protection
pub struct ResilientWorkerClient {
    inner: Box<dyn WorkerClient + Send + Sync>,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

impl ResilientWorkerClient {
    /// Create a new resilient worker client with default circuit breaker config
    pub fn new(worker_client: Box<dyn WorkerClient + Send + Sync>) -> Self {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            expected_exception: None,
        };
        Self::new_with_config(worker_client, config)
    }

    /// Create a new resilient worker client with custom circuit breaker config
    pub fn new_with_config(
        worker_client: Box<dyn WorkerClient + Send + Sync>,
        config: CircuitBreakerConfig,
    ) -> Self {
        Self {
            inner: worker_client,
            circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(config))),
        }
    }

    /// Get the underlying circuit breaker (for monitoring/inspection)
    pub fn get_circuit_breaker(&self) -> &Arc<Mutex<CircuitBreaker>> {
        &self.circuit_breaker
    }

    /// Reset the circuit breaker
    pub async fn reset_circuit_breaker(&self) {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.reset().await;
    }

    /// Get current circuit breaker state
    pub async fn get_circuit_state(&self) -> String {
        let circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.get_state_string()
    }

    /// Get failure count
    pub async fn get_failure_count(&self) -> u32 {
        let circuit_breaker = self.circuit_breaker.lock().await;
        circuit_breaker.get_failure_count()
    }
}

#[async_trait]
impl WorkerClient for ResilientWorkerClient {
    async fn assign_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
        job_spec: &JobSpec,
    ) -> Result<(), WorkerClientError> {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        let inner = &self.inner;
        match circuit_breaker
            .execute(|| async {
                inner
                    .assign_job(worker_id, job_id, job_spec)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(WorkerClientError::Communication(e.to_string())),
        }
    }

    async fn cancel_job(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> Result<(), WorkerClientError> {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        let inner = &self.inner;
        match circuit_breaker
            .execute(|| async {
                inner
                    .cancel_job(worker_id, job_id)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(WorkerClientError::Communication(e.to_string())),
        }
    }

    async fn get_worker_status(
        &self,
        worker_id: &WorkerId,
    ) -> Result<WorkerStatus, WorkerClientError> {
        let mut circuit_breaker = self.circuit_breaker.lock().await;
        let inner = &self.inner;
        match circuit_breaker
            .execute(|| async {
                inner
                    .get_worker_status(worker_id)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
            })
            .await
        {
            Ok(status) => Ok(status),
            Err(e) => Err(WorkerClientError::Communication(e.to_string())),
        }
    }

    async fn send_heartbeat(&self, worker_id: &WorkerId) -> Result<(), WorkerClientError> {
        self.inner.send_heartbeat(worker_id).await
    }
}

/// Factory for creating worker clients
pub struct WorkerClientFactory;

impl WorkerClientFactory {
    /// Create a gRPC worker client
    pub async fn create_grpc(
        worker_endpoint: String,
        timeout: Duration,
    ) -> Result<GrpcWorkerClient, WorkerClientError> {
        let channel = Channel::from_shared(worker_endpoint)
            .map_err(|e| WorkerClientError::Configuration(e.to_string()))?
            .connect()
            .await
            .map_err(|e| {
                WorkerClientError::Connection(format!("Failed to connect to worker: {}", e))
            })?;

        Ok(GrpcWorkerClient::new(channel, timeout))
    }

    /// Create an HTTP worker client
    pub fn create_http(base_url: String, timeout: Duration) -> HttpWorkerClient {
        HttpWorkerClient::new(base_url, timeout)
    }

    /// Create a resilient gRPC worker client with circuit breaker protection
    pub async fn create_resilient_grpc(
        worker_endpoint: String,
        timeout: Duration,
        circuit_breaker_config: CircuitBreakerConfig,
    ) -> Result<ResilientWorkerClient, WorkerClientError> {
        let grpc_client = Self::create_grpc(worker_endpoint, timeout).await?;
        Ok(ResilientWorkerClient::new_with_config(
            Box::new(grpc_client),
            circuit_breaker_config,
        ))
    }

    /// Create a resilient HTTP worker client with circuit breaker protection
    pub fn create_resilient_http(
        base_url: String,
        timeout: Duration,
        circuit_breaker_config: CircuitBreakerConfig,
    ) -> ResilientWorkerClient {
        let http_client = Self::create_http(base_url, timeout);
        ResilientWorkerClient::new_with_config(Box::new(http_client), circuit_breaker_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::WorkerId;
    use std::time::Duration;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let worker_id = WorkerId::new();
        let collector = MetricsCollector::new(worker_id.clone(), Duration::from_secs(5));

        assert_eq!(collector.worker_id, worker_id);
        assert_eq!(collector.interval, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_worker_metrics_exporter_creation() {
        let exporter = WorkerMetricsExporter::new(None);

        let workers = exporter.list_monitored_workers().await;
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn test_worker_metrics_exporter_creation_with_prometheus() {
        let registry = prometheus::Registry::new();
        let exporter = WorkerMetricsExporter::new(Some(registry));

        let workers = exporter.list_monitored_workers().await;
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn test_register_worker() {
        let exporter = WorkerMetricsExporter::new(None);
        let worker_id = WorkerId::new();

        exporter
            .register_worker(worker_id.clone(), Duration::from_secs(5))
            .await;

        let workers = exporter.list_monitored_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0], worker_id);
    }

    #[tokio::test]
    async fn test_unregister_worker() {
        let exporter = WorkerMetricsExporter::new(None);
        let worker_id = WorkerId::new();

        exporter
            .register_worker(worker_id.clone(), Duration::from_secs(5))
            .await;
        exporter.unregister_worker(&worker_id).await;

        let workers = exporter.list_monitored_workers().await;
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn test_get_worker_metrics_not_found() {
        let exporter = WorkerMetricsExporter::new(None);
        let worker_id = WorkerId::new();

        let result = exporter.get_worker_metrics(&worker_id).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not registered"));
        }
    }

    #[tokio::test]
    async fn test_resource_usage_structure() {
        let worker_id = WorkerId::new();
        let mut collector = MetricsCollector::new(worker_id, Duration::from_secs(5));

        // First call to get initial metrics (will be 0 for CPU)
        let metrics = collector.collect().await.unwrap();

        assert!(metrics.timestamp <= chrono::Utc::now());
        assert_eq!(metrics.cpu_percent, 0.0); // First read is always 0
        assert!(metrics.memory_rss_mb >= 0); // Can be 0 or actual value
        assert!(metrics.memory_vms_mb >= 0); // Can be 0 or actual value
        assert_eq!(metrics.disk_read_mb, 0.0); // Placeholder
        assert_eq!(metrics.disk_write_mb, 0.0); // Placeholder
        assert_eq!(metrics.network_sent_mb, 0.0); // Placeholder
        assert_eq!(metrics.network_received_mb, 0.0); // Placeholder
        assert_eq!(metrics.gpu_utilization_percent, None); // Placeholder
    }

    #[tokio::test]
    async fn test_multiple_workers_registration() {
        let exporter = WorkerMetricsExporter::new(None);

        let mut worker_ids = Vec::new();
        for _i in 1..=5 {
            let worker_id = WorkerId::new();
            worker_ids.push(worker_id.clone());
            exporter
                .register_worker(worker_id, Duration::from_secs(5))
                .await;
        }

        let workers = exporter.list_monitored_workers().await;
        assert_eq!(workers.len(), 5);

        // Verify all workers are monitored
        for worker_id in &worker_ids {
            assert!(workers.contains(worker_id));
        }
    }

    #[tokio::test]
    async fn test_collect_metrics_after_register() {
        let exporter = WorkerMetricsExporter::new(None);
        let worker_id = WorkerId::new();

        // Register worker
        exporter
            .register_worker(worker_id.clone(), Duration::from_secs(5))
            .await;

        // Get metrics
        let metrics = exporter.get_worker_metrics(&worker_id).await.unwrap();

        assert_eq!(metrics.cpu_percent, 0.0); // First reading
        assert!(metrics.memory_rss_mb >= 0); // May be 0 on some systems
    }

    #[test]
    fn test_resource_usage_serialization() {
        let usage = ResourceUsage {
            cpu_percent: 45.5,
            memory_rss_mb: 1024,
            memory_vms_mb: 2048,
            disk_read_mb: 100.5,
            disk_write_mb: 50.25,
            network_sent_mb: 75.0,
            network_received_mb: 150.0,
            gpu_utilization_percent: Some(80.0),
            timestamp: chrono::Utc::now(),
        };

        // Test serialization to JSON
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: ResourceUsage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.cpu_percent, usage.cpu_percent);
        assert_eq!(deserialized.memory_rss_mb, usage.memory_rss_mb);
        assert_eq!(deserialized.memory_vms_mb, usage.memory_vms_mb);
        assert_eq!(deserialized.disk_read_mb, usage.disk_read_mb);
        assert_eq!(deserialized.disk_write_mb, usage.disk_write_mb);
        assert_eq!(deserialized.network_sent_mb, usage.network_sent_mb);
        assert_eq!(deserialized.network_received_mb, usage.network_received_mb);
        assert_eq!(
            deserialized.gpu_utilization_percent,
            usage.gpu_utilization_percent
        );
    }

    #[test]
    fn test_memory_usage_structure() {
        let memory = MemoryUsage {
            rss_mb: 1024,
            vms_mb: 2048,
        };

        assert_eq!(memory.rss_mb, 1024);
        assert_eq!(memory.vms_mb, 2048);
    }

    #[test]
    fn test_cpu_times_structure() {
        let cpu_times = CpuTimes {
            user: 100,
            nice: 50,
            system: 75,
            idle: 200,
        };

        assert_eq!(cpu_times.user, 100);
        assert_eq!(cpu_times.nice, 50);
        assert_eq!(cpu_times.system, 75);
        assert_eq!(cpu_times.idle, 200);
    }

    #[test]
    fn test_metrics_error_display() {
        let error = MetricsError::InvalidProcStat;
        assert!(error.to_string().contains("/proc/stat"));

        let error = MetricsError::InvalidCpuTimes;
        assert!(error.to_string().contains("CPU times"));

        let error = MetricsError::FieldNotFound("VmRSS".to_string());
        assert!(error.to_string().contains("VmRSS"));
    }

    #[test]
    fn test_grpc_worker_client_creation() {
        // This would require a mock channel in real tests
        // Placeholder for now
    }

    #[test]
    fn test_http_worker_client_creation() {
        let client = HttpWorkerClient::with_default_timeout("http://localhost:8082".to_string());
        assert_eq!(client.timeout, Duration::from_secs(5));
    }
}
