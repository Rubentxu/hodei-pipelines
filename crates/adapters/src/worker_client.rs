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
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tonic::transport::Channel;
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
        let diskstats_content = tokio::fs::read_to_string("/proc/diskstats")
            .await
            .map_err(MetricsError::SystemReadError)?;

        let mut total_read_sectors = 0u64;
        let mut total_write_sectors = 0u64;

        for line in diskstats_content.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();

            // /proc/diskstats format: device name, read I/Os, read merges, read sectors, read time (ms),
            // write I/Os, write merges, write sectors, write time, I/O time, weighted I/O time
            if fields.len() >= 14 {
                let device = fields[2];

                // Skip loop devices and ram disks
                if device.starts_with("loop") || device.starts_with("ram") {
                    continue;
                }

                // Parse read sectors (field 3, 0-indexed: 3) and write sectors (field 7)
                if let (Ok(read_sectors), Ok(write_sectors)) =
                    (fields[3].parse::<u64>(), fields[7].parse::<u64>())
                {
                    // Assume 512 bytes per sector (standard for most Linux systems)
                    total_read_sectors += read_sectors;
                    total_write_sectors += write_sectors;
                }
            }
        }

        // Convert sectors to MB (512 bytes/sector * sectors / 1024 / 1024 = MB)
        let disk_read_mb = total_read_sectors as f64 * 512.0 / (1024.0 * 1024.0);
        let disk_write_mb = total_write_sectors as f64 * 512.0 / (1024.0 * 1024.0);

        Ok((disk_read_mb, disk_write_mb))
    }

    /// Collect network I/O metrics
    async fn collect_network_usage(&self) -> Result<(f64, f64), MetricsError> {
        let netdev_content = tokio::fs::read_to_string("/proc/net/dev")
            .await
            .map_err(MetricsError::SystemReadError)?;

        let mut total_rx_bytes = 0u64;
        let mut total_tx_bytes = 0u64;

        for line in netdev_content.lines() {
            // Skip header lines
            if line.contains("Inter-|") || line.contains(" face|") {
                continue;
            }

            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let interface = parts[0].trim();
            let metrics: Vec<&str> = parts[1].split_whitespace().collect();

            // Skip loopback interface
            if interface == "lo" {
                continue;
            }

            // /proc/net/dev format: bytes received, packets received, err in, drop in, ...
            // bytes transmitted, packets transmitted, err out, drop out, ...
            if metrics.len() >= 16
                && let (Ok(rx_bytes), Ok(tx_bytes)) = (
                    metrics[0].parse::<u64>(), // bytes received
                    metrics[8].parse::<u64>(), // bytes transmitted
                ) {
                    total_rx_bytes += rx_bytes;
                    total_tx_bytes += tx_bytes;
                }
        }

        // Convert bytes to MB
        let network_received_mb = total_rx_bytes as f64 / (1024.0 * 1024.0);
        let network_sent_mb = total_tx_bytes as f64 / (1024.0 * 1024.0);

        Ok((network_sent_mb, network_received_mb))
    }

    /// Collect GPU utilization (optional)
    async fn collect_gpu_usage(&self) -> Result<Option<f64>, MetricsError> {
        // Try to detect and collect GPU metrics using nvidia-smi
        match self.try_nvidia_gpu_metrics().await {
            Ok(Some(utilization)) => return Ok(Some(utilization)),
            Ok(None) => { /* Try other methods */ }
            Err(e) => {
                // If nvidia-smi fails, log debug but don't fail
                debug!("Failed to collect GPU metrics via nvidia-smi: {}", e);
            }
        }

        // Try to detect AMD GPUs via sysfs
        match self.try_amd_gpu_metrics().await {
            Ok(Some(utilization)) => return Ok(Some(utilization)),
            Ok(None) => { /* No AMD GPU found */ }
            Err(e) => {
                debug!("Failed to collect AMD GPU metrics: {}", e);
            }
        }

        // No GPU detected or GPU metrics unavailable
        Ok(None)
    }

    /// Try to collect NVIDIA GPU metrics using nvidia-smi
    async fn try_nvidia_gpu_metrics(&self) -> Result<Option<f64>, MetricsError> {
        // Check if nvidia-smi is available
        let output = tokio::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .await
            .map_err(|e| MetricsError::Other(format!("Failed to execute nvidia-smi: {}", e)))?;

        if !output.status.success() {
            return Ok(None); // nvidia-smi not available or failed
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let utilization_str = stdout.trim();

        // Parse the utilization percentage
        if let Ok(utilization) = utilization_str.parse::<f64>() {
            return Ok(Some(utilization));
        }

        Ok(None)
    }

    /// Try to collect AMD GPU metrics from sysfs
    async fn try_amd_gpu_metrics(&self) -> Result<Option<f64>, MetricsError> {
        // Check for AMD GPU directories in /sys/class/drm
        let drm_path = std::path::Path::new("/sys/class/drm");
        if !drm_path.exists() {
            return Ok(None);
        }

        let entries = tokio::fs::read_dir(drm_path)
            .await
            .map_err(|e| MetricsError::Other(format!("Failed to read /sys/class/drm: {}", e)))?;

        let mut amdgpu_paths = Vec::new();

        let mut entries = entries;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| MetricsError::Other(format!("Failed to read directory entry: {}", e)))?
        {
            if let Ok(path) = entry
                .path()
                .to_str()
                .ok_or(MetricsError::Other("Invalid path".to_string()))
                .map(|s| s.to_string())
                && path.contains("card") {
                    // Check if it's an AMD GPU by looking for vendor file
                    let vendor_path = format!("{}/device/vendor", path);
                    if let Ok(vendor) = tokio::fs::read_to_string(&vendor_path).await
                        && vendor.trim() == "0x1002" {
                            // AMD vendor ID
                            amdgpu_paths.push(path);
                        }
                }
        }

        if amdgpu_paths.is_empty() {
            return Ok(None);
        }

        // Try to read GPU utilization from sysfs (this varies by driver version)
        // For now, return None as AMD GPU metrics collection is complex
        // and driver-dependent
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

        let _result = timeout(self.timeout, async {
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

        let _result = timeout(self.timeout, async {
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
        // Collect resource metrics for this heartbeat
        let mut metrics_collector =
            MetricsCollector::new(worker_id.clone(), Duration::from_secs(10));
        let resource_usage = match metrics_collector.collect().await {
            Ok(usage) => {
                // Convert our internal ResourceUsage to proto format
                Some(hwp_proto::ResourceUsage {
                    cpu_usage_m: usage.cpu_percent as u64,
                    memory_usage_mb: usage.memory_rss_mb,
                    active_jobs: 0, // TODO: Get from worker
                    disk_read_mb: usage.disk_read_mb,
                    disk_write_mb: usage.disk_write_mb,
                    network_sent_mb: usage.network_sent_mb,
                    network_received_mb: usage.network_received_mb,
                    gpu_utilization_percent: usage.gpu_utilization_percent.unwrap_or(0.0),
                    timestamp: usage.timestamp.timestamp_nanos_opt().unwrap_or(0),
                })
            }
            Err(e) => {
                // If metrics collection fails, log warning but continue without metrics
                warn!(
                    "Failed to collect resource metrics for worker {}: {}",
                    worker_id, e
                );
                None
            }
        };

        let request = hwp_proto::HeartbeatRequest {
            worker_id: worker_id.to_string(),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            resource_usage,
        };

        let _result = timeout(self.timeout, async {
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

        let _response = timeout(self.timeout, async {
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

