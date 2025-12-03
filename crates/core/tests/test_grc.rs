//! Tests for Global Resource Controller (GRC)

use hodei_pipelines_core::resource_governance::*;
use std::collections::HashMap;

#[test]
fn test_grc_pool_selection_bin_packing() {
    let grc = create_test_grc();

    // Create pools with different utilizations
    let mut pool1 = create_pool("pool-1", 16000, 32768, 4, 100, 10); // 10% utilized
    let mut pool2 = create_pool("pool-2", 16000, 32768, 4, 100, 60); // 60% utilized (optimal)
    let mut pool3 = create_pool("pool-3", 16000, 32768, 4, 100, 90); // 90% utilized (over-utilized)

    grc.register_pool(pool1.clone()).await.unwrap();
    grc.register_pool(pool2.clone()).await.unwrap();
    grc.register_pool(pool3.clone()).await.unwrap();

    // Create a resource request
    let mut labels = HashMap::new();
    labels.insert("environment".to_string(), "production".to_string());

    let request = ResourceRequest::builder()
        .request_id("req-1".into())
        .cpu_millicores(4000)
        .memory_mb(8192)
        .gpu_count(Some(1))
        .required_labels(labels)
        .priority(5)
        .build()
        .unwrap();

    // Select best pool - should prefer pool2 (optimal utilization)
    let selected_pool = grc
        .select_best_pool(&[pool1, pool2, pool3], &request)
        .await
        .unwrap();

    assert_eq!(selected_pool.id.as_str(), "pool-2");
}

#[test]
fn test_grc_cost_aware_scheduling() {
    let grc = create_test_grc();

    // Pool 1: Low cost
    let mut pool1 = create_pool_with_cost("pool-1", 8000, 16384, 2, 50, 10, 100); // $0.01/core/hour
    // Pool 2: High cost
    let mut pool2 = create_pool_with_cost("pool-2", 8000, 16384, 2, 50, 10, 1000); // $0.10/core/hour

    grc.register_pool(pool1.clone()).await.unwrap();
    grc.register_pool(pool2.clone()).await.unwrap();

    let request = ResourceRequest::builder()
        .request_id("req-2".into())
        .cpu_millicores(2000)
        .memory_mb(4096)
        .priority(5)
        .build()
        .unwrap();

    // Should prefer pool1 (lower cost)
    let selected_pool = grc
        .select_best_pool(&[pool1, pool2], &request)
        .await
        .unwrap();

    assert_eq!(selected_pool.id.as_str(), "pool-1");
}

#[test]
fn test_grc_label_based_filtering() {
    let grc = create_test_grc();

    let mut pool1 = create_pool_with_labels(
        "pool-1",
        &[("environment", "production"), ("region", "us-east")],
    );
    let mut pool2 = create_pool_with_labels(
        "pool-2",
        &[("environment", "development"), ("region", "us-east")],
    );
    let mut pool3 = create_pool_with_labels(
        "pool-3",
        &[("environment", "production"), ("region", "us-west")],
    );

    grc.register_pool(pool1.clone()).await.unwrap();
    grc.register_pool(pool2.clone()).await.unwrap();
    grc.register_pool(pool3.clone()).await.unwrap();

    // Request only production pools in us-east
    let mut required_labels = HashMap::new();
    required_labels.insert("environment".to_string(), "production".to_string());
    required_labels.insert("region".to_string(), "us-east".to_string());

    let request = ResourceRequest::builder()
        .request_id("req-3".into())
        .cpu_millicores(2000)
        .memory_mb(4096)
        .required_labels(required_labels)
        .priority(5)
        .build()
        .unwrap();

    let candidate_pools = grc.find_candidate_pools(&request).await.unwrap();

    // Should only find pool1
    assert_eq!(candidate_pools.len(), 1);
    assert_eq!(candidate_pools[0].id.as_str(), "pool-1");
}

#[test]
fn test_grc_resource_allocation() {
    let grc = create_test_grc();

    let pool = create_pool("pool-1", 16000, 32768, 4, 100, 0);
    grc.register_pool(pool.clone()).await.unwrap();

    let request = ResourceRequest::builder()
        .request_id("alloc-1".into())
        .cpu_millicores(4000)
        .memory_mb(8192)
        .gpu_count(Some(1))
        .priority(5)
        .build()
        .unwrap();

    // Allocate resources
    let allocation = grc
        .allocate_resources("alloc-1".to_string(), "pool-1".into(), request.clone())
        .await
        .unwrap();

    assert_eq!(allocation.pool_id.as_str(), "pool-1");
    assert_eq!(allocation.resources.cpu_millicores, 4000);
    assert_eq!(allocation.resources.memory_mb, 8192);
    assert_eq!(allocation.resources.gpu_count, 1);

    // Check pool capacity is updated
    let pool_status = grc.get_pool_status("pool-1").await.unwrap();
    assert_eq!(pool_status.used_capacity.cpu_millicores, 4000);
    assert_eq!(pool_status.used_capacity.memory_mb, 8192);
}

#[test]
fn test_grc_allocation_release() {
    let grc = create_test_grc();

    let pool = create_pool("pool-1", 16000, 32768, 4, 100, 0);
    grc.register_pool(pool.clone()).await.unwrap();

    let request = ResourceRequest::builder()
        .request_id("alloc-2".into())
        .cpu_millicores(4000)
        .memory_mb(8192)
        .priority(5)
        .build()
        .unwrap();

    // Allocate
    grc.allocate_resources("alloc-2".to_string(), "pool-1".into(), request.clone())
        .await
        .unwrap();

    // Release
    grc.release_resources("alloc-2").await.unwrap();

    // Check pool capacity is restored
    let pool_status = grc.get_pool_status("pool-1").await.unwrap();
    assert_eq!(pool_status.used_capacity.cpu_millicores, 0);
    assert_eq!(pool_status.used_capacity.memory_mb, 0);
}

#[test]
fn test_grc_insufficient_capacity() {
    let grc = create_test_grc();

    let pool = create_pool("pool-1", 2000, 4096, 0, 10, 0); // Small pool
    grc.register_pool(pool.clone()).await.unwrap();

    let request = ResourceRequest::builder()
        .request_id("req-4".into())
        .cpu_millicores(4000) // Exceeds pool capacity
        .memory_mb(8192) // Exceeds pool capacity
        .priority(5)
        .build()
        .unwrap();

    // Should fail to allocate
    let result = grc
        .allocate_resources("alloc-3".to_string(), "pool-1".into(), request)
        .await;

    assert!(result.is_err());
}

#[test]
fn test_grc_quota_enforcement() {
    let grc = create_test_grc();

    // Set up tenant quota
    let quota = TenantQuota::builder()
        .tenant_id("tenant-1".to_string())
        .max_cpu_cores(8) // 8000m
        .max_memory_mb(16384)
        .max_concurrent_jobs(5)
        .build()
        .unwrap();

    grc.register_tenant_quota("tenant-1".to_string(), quota.clone())
        .await
        .unwrap();

    let mut pool = create_pool("pool-1", 16000, 32768, 4, 100, 0);
    grc.register_pool(pool.clone()).await.unwrap();

    // Request within quota (6 cores)
    let request1 = ResourceRequest::builder()
        .request_id("req-5".into())
        .cpu_millicores(6000)
        .memory_mb(8192)
        .tenant_id("tenant-1".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result1 = grc
        .allocate_with_quota_check("alloc-4".to_string(), "pool-1".into(), request1)
        .await;
    assert!(result1.is_ok());

    // Request exceeding quota (10 cores > 8 cores limit)
    let request2 = ResourceRequest::builder()
        .request_id("req-6".into())
        .cpu_millicores(10000)
        .memory_mb(8192)
        .tenant_id("tenant-1".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result2 = grc
        .allocate_with_quota_check("alloc-5".to_string(), "pool-1".into(), request2)
        .await;
    assert!(result2.is_err());
}

fn create_test_grc() -> GlobalResourceController {
    GlobalResourceController::new(GRCConfig::default())
}

fn create_pool(
    id: &str,
    cpu_millicores: u64,
    memory_mb: u64,
    gpu_count: u32,
    max_workers: u32,
    active_workers: u32,
) -> ComputePool {
    ComputePool::builder()
        .id(id.into())
        .name(format!("Pool {}", id))
        .provider_type(ProviderType::Kubernetes)
        .total_capacity(PoolCapacity {
            cpu_millicores,
            memory_mb,
            gpu_count,
            max_workers,
            active_workers: 0,
            storage_gb: Some(500),
        })
        .used_capacity(PoolCapacity {
            cpu_millicores: (active_workers as f64 * (cpu_millicores as f64 / max_workers as f64))
                as u64,
            memory_mb: (active_workers as f64 * (memory_mb as f64 / max_workers as f64)) as u64,
            gpu_count: (active_workers as f64 * (gpu_count as f64 / max_workers as f64)) as u32,
            max_workers,
            active_workers,
            storage_gb: Some(500),
        })
        .build()
        .unwrap()
}

fn create_pool_with_cost(
    id: &str,
    cpu_millicores: u64,
    memory_mb: u64,
    gpu_count: u32,
    max_workers: u32,
    active_workers: u32,
    cpu_hour_cents: u64,
) -> ComputePool {
    let mut pool = create_pool(
        id,
        cpu_millicores,
        memory_mb,
        gpu_count,
        max_workers,
        active_workers,
    );
    pool.cost_config = Some(CostConfig {
        cpu_hour_cents,
        memory_gb_hour_cents: 1,
        gpu_hour_cents: 100,
        storage_gb_hour_cents: 1,
        base_hourly_cents: 10,
    });
    pool
}

fn create_pool_with_labels(id: &str, labels: &[(&str, &str)]) -> ComputePool {
    let mut labels_map = HashMap::new();
    for (k, v) in labels {
        labels_map.insert(k.to_string(), v.to_string());
    }

    ComputePool::builder()
        .id(id.into())
        .name(format!("Pool {}", id))
        .provider_type(ProviderType::Kubernetes)
        .total_capacity(PoolCapacity {
            cpu_millicores: 8000,
            memory_mb: 16384,
            gpu_count: 2,
            max_workers: 50,
            active_workers: 0,
            storage_gb: Some(500),
        })
        .labels(labels_map)
        .build()
        .unwrap()
}
