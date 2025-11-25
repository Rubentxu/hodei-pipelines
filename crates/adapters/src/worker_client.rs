//! Production-grade Worker Client Implementation
//!
//! This module provides real gRPC and HTTP client implementations for
//! worker communication, replacing mock implementations for production use.

use async_trait::async_trait;
use chrono;
use hodei_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use hodei_core::{JobId, JobSpec};
use hodei_core::{WorkerId, WorkerStatus};
use hodei_ports::{WorkerClient, WorkerClientError};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tonic::{Status, transport::Channel};
use tracing::{debug, error, info, warn};

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
