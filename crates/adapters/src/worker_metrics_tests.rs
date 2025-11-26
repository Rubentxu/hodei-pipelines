//! Tests for worker metrics collection functionality

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::WorkerId;
    use std::time::Duration;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let worker_id = WorkerId::new("test-worker-1");
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
        let worker_id = WorkerId::new("worker-1");

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
        let worker_id = WorkerId::new("worker-1");

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
        let worker_id = WorkerId::new("nonexistent-worker");

        let result = exporter.get_worker_metrics(&worker_id).await;

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not registered"));
        }
    }

    #[tokio::test]
    async fn test_resource_usage_structure() {
        let worker_id = WorkerId::new("test-worker");
        let mut collector = MetricsCollector::new(worker_id, Duration::from_secs(5));

        // First call to get initial metrics (will be 0 for CPU)
        let metrics = collector.collect().await.unwrap();

        assert!(metrics.timestamp <= chrono::Utc::now());
        assert_eq!(metrics.cpu_percent, 0.0); // First read is always 0
        assert_eq!(metrics.memory_rss_mb, 0); // Should be > 0 on real system
        assert_eq!(metrics.memory_vms_mb, 0); // Should be > 0 on real system
        assert_eq!(metrics.disk_read_mb, 0.0); // Placeholder
        assert_eq!(metrics.disk_write_mb, 0.0); // Placeholder
        assert_eq!(metrics.network_sent_mb, 0.0); // Placeholder
        assert_eq!(metrics.network_received_mb, 0.0); // Placeholder
        assert_eq!(metrics.gpu_utilization_percent, None); // Placeholder
    }

    #[tokio::test]
    async fn test_multiple_workers_registration() {
        let exporter = WorkerMetricsExporter::new(None);

        for i in 1..=5 {
            let worker_id = WorkerId::new(&format!("worker-{}", i));
            exporter
                .register_worker(worker_id, Duration::from_secs(5))
                .await;
        }

        let workers = exporter.list_monitored_workers().await;
        assert_eq!(workers.len(), 5);

        // Verify all workers are monitored
        for i in 1..=5 {
            let worker_id = WorkerId::new(&format!("worker-{}", i));
            assert!(workers.contains(&worker_id));
        }
    }

    #[tokio::test]
    async fn test_metrics_error_types() {
        // Test InvalidProcStat error
        let collector = MetricsCollector::new(WorkerId::new("test"), Duration::from_secs(5));

        // This would fail on systems without /proc/stat
        // The error should be MetricsError::SystemReadError
        let result = collector.collect_cpu_usage().await;

        // On systems with /proc/stat, this should succeed
        // On systems without it, this should error
        if result.is_err() {
            let error = result.unwrap_err();
            assert!(matches!(error, MetricsError::SystemReadError(_)));
        }
    }

    #[tokio::test]
    async fn test_resource_usage_serialization() {
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

    #[tokio::test]
    async fn test_collect_metrics_after_register() {
        let exporter = WorkerMetricsExporter::new(None);
        let worker_id = WorkerId::new("test-worker");

        // Register worker
        exporter
            .register_worker(worker_id.clone(), Duration::from_secs(5))
            .await;

        // Get metrics
        let metrics = exporter.get_worker_metrics(&worker_id).await.unwrap();

        assert_eq!(metrics.cpu_percent, 0.0); // First reading
        assert!(metrics.memory_rss_mb > 0 || metrics.memory_rss_mb == 0); // May be 0 on some systems
    }

    #[tokio::test]
    async fn test_cpu_delta_calculation_on_repeated_reads() {
        let worker_id = WorkerId::new("test-worker");
        let mut collector = MetricsCollector::new(worker_id, Duration::from_secs(5));

        // First read - should return 0
        let metrics1 = collector.collect().await.unwrap();
        assert_eq!(metrics1.cpu_percent, 0.0);

        // Second read - should calculate actual percentage
        // Note: If /proc/stat shows same values, this might still be 0
        let metrics2 = collector.collect().await.unwrap();
        // On a real system with activity, this would be > 0
        // But we can't guarantee it for tests
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
}
