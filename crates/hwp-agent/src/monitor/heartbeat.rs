//! Heartbeat module
//!
//! This module sends periodic heartbeat messages to the server
//! with resource usage and status updates.

use hodei_core::WorkerId;
use tokio::time::{Duration, MissedTickBehavior, interval};
use tracing::{error, info, warn};

use super::resources::ResourceMonitor;
use crate::{AgentError, Result};

use super::resources::ResourceUsage;
use hodei_ports::WorkerClient;

/// Heartbeat configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    pub interval_ms: u64,
    pub max_failures: u32,
    pub failure_timeout_ms: u64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5000,
            max_failures: 3,
            failure_timeout_ms: 30000,
        }
    }
}

/// Heartbeat sender with real gRPC implementation
pub struct HeartbeatSender {
    config: HeartbeatConfig,
    resource_monitor: ResourceMonitor,
    worker_id: WorkerId,
    worker_client: Box<dyn WorkerClient>,
}

impl HeartbeatSender {
    /// Create a new heartbeat sender
    pub fn new(
        config: HeartbeatConfig,
        resource_monitor: ResourceMonitor,
        worker_id: WorkerId,
        worker_client: Box<dyn WorkerClient>,
    ) -> Self {
        Self {
            config,
            resource_monitor,
            worker_id,
            worker_client,
        }
    }

    /// Start sending heartbeats with real gRPC implementation
    pub async fn start(&mut self, pids: Vec<u32>) -> Result<()> {
        info!(
            "Starting heartbeat sender with interval {}ms for worker {}",
            self.config.interval_ms, self.worker_id
        );

        let mut interval = interval(Duration::from_millis(self.config.interval_ms));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let mut failure_count = 0;

        // Channel for receiving resource updates
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ResourceUsage>(100);

        // Spawn monitoring task
        let monitor = self.resource_monitor.clone();
        tokio::spawn(async move {
            monitor.monitor_pids(&pids, tx).await;
        });

        loop {
            interval.tick().await;

            // Receive resource usage
            match rx.recv().await {
                Some(usage) => {
                    // Send heartbeat message via gRPC with retry logic
                    let usage_clone = usage.clone();
                    let send_result = self.send_heartbeat_with_retry(usage_clone).await;

                    match send_result {
                        Ok(_) => {
                            info!(
                                "Heartbeat sent successfully: PID {}, CPU {:.2}%, MEM {} MB",
                                usage.pid,
                                usage.cpu_percent,
                                usage.memory_bytes / 1024 / 1024
                            );
                            failure_count = 0; // Reset failure count on success
                        }
                        Err(e) => {
                            failure_count += 1;
                            error!(
                                "Failed to send heartbeat (attempt {}): {}",
                                failure_count, e
                            );

                            if failure_count >= self.config.max_failures {
                                error!("Max heartbeat failures reached, stopping heartbeat sender");
                                break;
                            }
                        }
                    }
                }
                None => {
                    failure_count += 1;
                    warn!(
                        "Failed to receive resource usage (attempt {})",
                        failure_count
                    );

                    if failure_count >= self.config.max_failures {
                        error!("Max heartbeat failures reached, stopping heartbeat sender");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Send heartbeat with exponential backoff retry logic
    async fn send_heartbeat_with_retry(&mut self, usage: ResourceUsage) -> Result<()> {
        let mut attempt = 0;
        let max_attempts = 3;

        loop {
            // Send heartbeat via gRPC
            let send_result = self.send_heartbeat(&usage).await;

            match send_result {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) if attempt < max_attempts => {
                    attempt += 1;
                    let backoff = Duration::from_secs(2u64.pow(attempt));
                    warn!(
                        "Heartbeat attempt {} failed: {}. Retrying in {:?}",
                        attempt, e, backoff
                    );
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => {
                    return Err(AgentError::Other(format!(
                        "Heartbeat failed after {} attempts: {}",
                        max_attempts, e
                    )));
                }
            }
        }
    }

    /// Send heartbeat via gRPC client
    async fn send_heartbeat(&mut self, usage: &ResourceUsage) -> Result<()> {
        // Send heartbeat using the WorkerClient
        self.worker_client
            .send_heartbeat(&self.worker_id)
            .await
            .map_err(|e| {
                warn!("Heartbeat gRPC error: {}", e);
                e
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval_ms, 5000);
        assert_eq!(config.max_failures, 3);
    }

    #[tokio::test]
    async fn test_heartbeat_sender_creation() {
        let worker_id = WorkerId::new();
        let resource_monitor = ResourceMonitor::new(1000);

        // Create a mock worker client for testing
        let worker_client = Box::new(MockWorkerClient::new());

        let sender = HeartbeatSender::new(
            HeartbeatConfig::default(),
            resource_monitor,
            worker_id,
            worker_client,
        );

        assert_eq!(sender.worker_id.to_string().len() > 0, true);
    }

    /// Mock WorkerClient for testing
    struct MockWorkerClient;

    impl MockWorkerClient {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl WorkerClient for MockWorkerClient {
        async fn assign_job(
            &self,
            _worker_id: &WorkerId,
            _job_id: &hodei_core::JobId,
            _job_spec: &hodei_core::JobSpec,
        ) -> std::result::Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }

        async fn cancel_job(
            &self,
            _worker_id: &WorkerId,
            _job_id: &hodei_core::JobId,
        ) -> std::result::Result<(), hodei_ports::WorkerClientError> {
            Ok(())
        }

        async fn get_worker_status(
            &self,
            _worker_id: &WorkerId,
        ) -> std::result::Result<hodei_core::WorkerStatus, hodei_ports::WorkerClientError> {
            Ok(hodei_core::WorkerStatus {
                worker_id: _worker_id.clone(),
                status: "IDLE".to_string(),
                current_jobs: vec![],
                last_heartbeat: std::time::SystemTime::now(),
            })
        }

        async fn send_heartbeat(
            &self,
            _worker_id: &WorkerId,
        ) -> std::result::Result<(), hodei_ports::WorkerClientError> {
            // Simulate successful heartbeat
            Ok(())
        }
    }
}
