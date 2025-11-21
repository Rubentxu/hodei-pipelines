//! Resource Metrics Module
//!
//! Provides system resource metrics collection (CPU, memory, disk, network).

use super::MetricsCollector;
use prometheus::{Gauge, IntCounter};
use std::sync::Arc;

/// Resource metrics collector
pub struct ResourceMetrics {
    // CPU metrics
    cpu_usage: Gauge,

    // Memory metrics
    memory_usage: Gauge,
    memory_total: Gauge,

    // Disk metrics
    disk_usage: Gauge,
    disk_total: Gauge,

    // Network metrics
    network_rx_bytes: IntCounter,
    network_tx_bytes: IntCounter,
}

impl ResourceMetrics {
    /// Create new resource metrics
    pub fn new(
        collector: Arc<MetricsCollector>,
    ) -> Result<Self, super::custom_metrics::MetricsError> {
        Ok(Self {
            cpu_usage: collector
                .register_gauge("system_cpu_usage_percent", "System CPU usage percentage")?,
            memory_usage: collector
                .register_gauge("system_memory_usage_bytes", "System memory usage in bytes")?,
            memory_total: collector
                .register_gauge("system_memory_total_bytes", "System total memory in bytes")?,
            disk_usage: collector
                .register_gauge("system_disk_usage_bytes", "System disk usage in bytes")?,
            disk_total: collector.register_gauge(
                "system_disk_total_bytes",
                "System total disk space in bytes",
            )?,
            network_rx_bytes: collector.register_counter(
                "system_network_rx_bytes_total",
                "Total network bytes received",
            )?,
            network_tx_bytes: collector.register_counter(
                "system_network_tx_bytes_total",
                "Total network bytes transmitted",
            )?,
        })
    }

    /// Update CPU usage
    pub fn set_cpu_usage(&self, usage: f64) {
        self.cpu_usage.set(usage);
    }

    /// Update memory metrics
    pub fn set_memory_usage(&self, used: f64, total: f64) {
        self.memory_usage.set(used);
        self.memory_total.set(total);
    }

    /// Update disk metrics
    pub fn set_disk_usage(&self, used: f64, total: f64) {
        self.disk_usage.set(used);
        self.disk_total.set(total);
    }

    /// Record network bytes received
    pub fn inc_network_rx(&self, bytes: u64) {
        self.network_rx_bytes.inc_by(bytes);
    }

    /// Record network bytes transmitted
    pub fn inc_network_tx(&self, bytes: u64) {
        self.network_tx_bytes.inc_by(bytes);
    }
}
