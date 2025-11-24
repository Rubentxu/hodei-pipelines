#[cfg(test)]
mod cluster_state_tests {
    use hodei_modules::scheduler::{ClusterState, ResourceUsage};
    use hodei_shared_types::{WorkerCapabilities, WorkerId};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cluster_state_registration() {
        let cluster = ClusterState::new();

        let worker_id = WorkerId::new();
        let mut labels = HashMap::new();
        labels.insert("runtime".to_string(), "docker".to_string());
        let capabilities = WorkerCapabilities {
            cpu_cores: 4,
            memory_gb: 8,
            gpu: None,
            labels,
            features: vec!["amd64".to_string()],
            max_concurrent_jobs: 1,
        };

        cluster
            .register_worker(&worker_id, capabilities)
            .await
            .unwrap();

        let worker = cluster.get_worker(&worker_id).await.unwrap().unwrap();
        assert_eq!(worker.id, worker_id);
    }

    #[tokio::test]
    async fn test_resource_usage_tracking() {
        let cluster = ClusterState::new();

        let worker_id = WorkerId::new();
        let capabilities = WorkerCapabilities {
            cpu_cores: 4,
            memory_gb: 8,
            gpu: None,
            labels: HashMap::new(),
            features: vec!["amd64".to_string()],
            max_concurrent_jobs: 1,
        };

        cluster
            .register_worker(&worker_id, capabilities)
            .await
            .unwrap();

        // Update resource usage
        let usage = ResourceUsage {
            cpu_percent: 75.0,
            memory_mb: 4096,
            io_percent: 20.0,
        };

        cluster
            .update_resource_usage(&worker_id, usage.clone())
            .await
            .unwrap();

        let worker = cluster.get_worker(&worker_id).await.unwrap().unwrap();
        assert_eq!(worker.usage.cpu_percent, 75.0);
        assert_eq!(worker.usage.memory_mb, 4096);
        assert_eq!(worker.usage.io_percent, 20.0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let cluster = Arc::new(ClusterState::new());

        let handles: Vec<_> = (0..100)
            .map(|i| {
                let cluster = cluster.clone();
                let worker_id = WorkerId::new();
                let capabilities = WorkerCapabilities {
                    cpu_cores: 4,
                    memory_gb: 8,
                    gpu: None,
                    labels: HashMap::new(),
                    features: vec!["amd64".to_string()],
                    max_concurrent_jobs: 1,
                };
                tokio::spawn(async move {
                    cluster
                        .register_worker(&worker_id, capabilities)
                        .await
                        .unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(cluster.worker_count().await, 100);
    }

    #[tokio::test]
    async fn test_worker_heartbeat() {
        let cluster = ClusterState::new();

        let worker_id = WorkerId::new();
        let capabilities = WorkerCapabilities {
            cpu_cores: 4,
            memory_gb: 8,
            gpu: None,
            labels: HashMap::new(),
            features: vec!["amd64".to_string()],
            max_concurrent_jobs: 1,
        };

        cluster
            .register_worker(&worker_id, capabilities)
            .await
            .unwrap();

        let worker = cluster.get_worker(&worker_id).await.unwrap().unwrap();
        assert!(worker.is_healthy());

        // Simulate stale heartbeat
        tokio::time::sleep(Duration::from_secs(31)).await; // Default timeout is 30s

        let worker = cluster.get_worker(&worker_id).await.unwrap().unwrap();
        assert!(!worker.is_healthy());
    }

    #[tokio::test]
    async fn test_worker_capacity_check() {
        let cluster = ClusterState::new();

        let worker_id = WorkerId::new();
        let capabilities = WorkerCapabilities {
            cpu_cores: 8,
            memory_gb: 16,
            gpu: None,
            labels: HashMap::new(),
            features: vec!["amd64".to_string()],
            max_concurrent_jobs: 1,
        };

        cluster
            .register_worker(&worker_id, capabilities)
            .await
            .unwrap();

        let worker = cluster.get_worker(&worker_id).await.unwrap().unwrap();

        // Check capacity
        assert!(worker.has_capacity(4, 8192)); // Should fit
        assert!(!worker.has_capacity(16, 32768)); // Should not fit
    }
}
