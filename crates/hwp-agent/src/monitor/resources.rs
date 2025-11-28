//! Resource monitoring module
//!
//! This module monitors CPU, memory, and I/O usage using sysinfo crate.

use std::sync::{Arc, Mutex};
use sysinfo::{Pid, System};
use tokio::time::{Duration, interval};
use tracing::{debug, error};

/// Resource usage metrics
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub pid: u32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

impl ResourceUsage {
    /// Create empty usage
    pub fn empty(pid: u32) -> Self {
        Self {
            pid,
            cpu_percent: 0.0,
            memory_bytes: 0,
            memory_percent: 0.0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
        }
    }
}

/// Resource monitor for tracking job usage
#[derive(Clone)]
pub struct ResourceMonitor {
    system: Arc<Mutex<System>>,
    sampling_interval_ms: u64,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(sampling_interval_ms: u64) -> Self {
        Self {
            system: Arc::new(Mutex::new(System::new_all())),
            sampling_interval_ms,
        }
    }

    /// Get resource usage for a specific PID
    pub async fn get_usage(&self, pid: u32) -> Result<ResourceUsage, String> {
        let mut system = self.system.lock().unwrap();

        // Refresh system info
        system.refresh_all();

        // Find process
        match system.process(Pid::from_u32(pid)) {
            Some(process) => {
                // Calculate memory percentage
                let memory_percent = if system.total_memory() > 0 {
                    (process.memory() as f64 / system.total_memory() as f64 * 100.0) as f32
                } else {
                    0.0
                };

                let usage = ResourceUsage {
                    pid,
                    cpu_percent: process.cpu_usage(),
                    memory_bytes: process.memory(),
                    memory_percent,
                    disk_read_bytes: 0, // sysinfo doesn't track disk I/O per-process in v0.30
                    disk_write_bytes: 0,
                    network_rx_bytes: 0, // sysinfo doesn't track network per-process
                    network_tx_bytes: 0,
                };

                debug!(
                    "Resource usage for PID {}: CPU {:.2}%, MEM {} MB ({:.1}%)",
                    pid,
                    usage.cpu_percent,
                    usage.memory_bytes / 1024 / 1024,
                    memory_percent
                );

                Ok(usage)
            }
            None => Err(format!("Process {} not found", pid)),
        }
    }

    /// Monitor multiple PIDs continuously
    pub async fn monitor_pids(
        &self,
        pids: &[u32],
        sender: tokio::sync::mpsc::Sender<ResourceUsage>,
    ) {
        let mut interval = interval(Duration::from_millis(self.sampling_interval_ms));

        loop {
            interval.tick().await;

            for pid in pids {
                match self.get_usage(*pid).await {
                    Ok(usage) => {
                        if sender.send(usage).await.is_err() {
                            error!("Failed to send resource usage");
                            return;
                        }
                    }
                    Err(e) => {
                        debug!("Failed to get usage for PID {}: {}", pid, e);
                    }
                }
            }
        }
    }

    /// Get system-wide statistics
    pub async fn get_system_stats(&self) -> SystemStats {
        let system = self.system.lock().unwrap();

        SystemStats {
            total_memory: system.total_memory(),
            available_memory: system.available_memory(),
            used_memory: system.used_memory(),
            cpu_count: sysinfo::System::physical_core_count().unwrap_or(0),
            up_time: sysinfo::System::uptime(),
        }
    }
}

/// System-wide statistics
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_memory: u64,
    pub available_memory: u64,
    pub used_memory: u64,
    pub cpu_count: usize,
    pub up_time: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_monitor_creation() {
        let monitor = ResourceMonitor::new(1000);
        let stats = monitor.get_system_stats().await;
        assert!(stats.total_memory > 0);
        assert!(stats.cpu_count > 0);
    }

    #[tokio::test]
    async fn test_get_current_process_usage() {
        let monitor = ResourceMonitor::new(1000);
        let current_pid = std::process::id();
        let usage = monitor.get_usage(current_pid).await;
        assert!(usage.is_ok());

        let usage = usage.unwrap();
        assert_eq!(usage.pid, current_pid);
    }
}
