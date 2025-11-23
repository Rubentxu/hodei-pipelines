//! Heartbeat module
//!
//! This module sends periodic heartbeat messages to the server
//! with resource usage and status updates.

use tokio::time::{Duration, MissedTickBehavior, interval};
use tracing::{error, info, warn};

use super::resources::ResourceMonitor;
use crate::Result;

use super::resources::ResourceUsage;

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

/// Heartbeat sender
pub struct HeartbeatSender {
    config: HeartbeatConfig,
    resource_monitor: ResourceMonitor,
    server_sender: Option<Box<dyn std::any::Any + Send + Sync>>,
}

impl HeartbeatSender {
    /// Create a new heartbeat sender
    pub fn new(config: HeartbeatConfig, resource_monitor: ResourceMonitor) -> Self {
        Self {
            config,
            resource_monitor,
            server_sender: None,
        }
    }

    /// Set the gRPC sender
    pub fn set_sender(&mut self, sender: Box<dyn std::any::Any + Send + Sync>) {
        self.server_sender = Some(sender);
    }

    /// Start sending heartbeats
    pub async fn start(&self, pids: Vec<u32>) -> Result<()> {
        info!(
            "Starting heartbeat sender with interval {}ms",
            self.config.interval_ms
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
                    // TODO: Send heartbeat message via gRPC
                    info!(
                        "Sending heartbeat: PID {}, CPU {:.2}%, MEM {} MB",
                        usage.pid,
                        usage.cpu_percent,
                        usage.memory_bytes / 1024 / 1024
                    );

                    failure_count = 0; // Reset failure count on success
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval_ms, 5000);
        assert_eq!(config.max_failures, 3);
    }
}
