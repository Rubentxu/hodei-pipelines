//! Tests for Resource Governance Types

use hodei_pipelines_core::resource_governance::*;
use std::collections::HashMap;

#[test]
fn test_compute_pool_creation() {
    let pool = ComputePool::builder()
        .id("pool-1".into())
        .name("Production K8s Cluster".to_string())
        .provider_type(ProviderType::Kubernetes)
        .total_capacity(PoolCapacity {
            cpu_millicores: 16000, // 16 cores
            memory_mb: 32768,      // 32GB
            gpu_count: 4,
            max_workers: 100,
            active_workers: 0,
            storage_gb: Some(1000),
        })
        .build()
        .expect("Failed to create ComputePool");

    assert_eq!(pool.id.as_str(), "pool-1");
    assert_eq!(pool.name, "Production K8s Cluster");
    assert_eq!(pool.provider_type, ProviderType::Kubernetes);
    assert_eq!(pool.total_capacity.cpu_millicores, 16000);
    assert_eq!(pool.status, PoolStatus::Active);
}

#[test]
fn test_compute_pool_available_capacity() {
    let mut pool = create_test_pool();

    // Initially no capacity used
    let available = pool.available_capacity();
    assert_eq!(available.cpu_millicores, 16000);
    assert_eq!(available.memory_mb, 32768);

    // Simulate resource usage
    pool.used_capacity = PoolCapacity {
        cpu_millicores: 4000,
        memory_mb: 8192,
        gpu_count: 1,
        max_workers: 100,
        active_workers: 20,
        storage_gb: Some(1000),
    };

    let available = pool.available_capacity();
    assert_eq!(available.cpu_millicores, 12000); // 16000 - 4000
    assert_eq!(available.memory_mb, 24576); // 32768 - 8192
    assert_eq!(available.gpu_count, 3); // 4 - 1
}

#[test]
fn test_compute_pool_utilization_percentage() {
    let mut pool = create_test_pool();

    pool.used_capacity = PoolCapacity {
        cpu_millicores: 8000, // 50% utilization
        memory_mb: 16384,     // 50% utilization
        gpu_count: 2,         // 50% utilization
        max_workers: 100,
        active_workers: 50, // 50% utilization
        storage_gb: Some(1000),
    };

    let utilization = pool.utilization_percentage();
    assert!((utilization - 50.0).abs() < 0.1);
}

#[test]
fn test_resource_request_creation() {
    let mut labels = HashMap::new();
    labels.insert("environment".to_string(), "production".to_string());
    labels.insert("region".to_string(), "us-east-1".to_string());

    let request = ResourceRequest::builder()
        .request_id("req-123".into())
        .pipeline_id("pipe-456".into())
        .cpu_millicores(2000)
        .memory_mb(4096)
        .gpu_count(Some(1))
        .required_labels(labels)
        .tenant_id("tenant-1".to_string())
        .priority(8)
        .build()
        .expect("Failed to create ResourceRequest");

    assert_eq!(request.request_id.as_str(), "req-123");
    assert_eq!(request.cpu_millicores, 2000);
    assert_eq!(request.memory_mb, 4096);
    assert_eq!(request.gpu_count, Some(1));
    assert_eq!(request.tenant_id.as_ref(), Some(&"tenant-1".to_string()));
    assert_eq!(request.priority, 8);
}

#[test]
fn test_tenant_quota_creation() {
    let mut allowed_pools = Vec::new();
    allowed_pools.push("pool-1".into());
    allowed_pools.push("pool-2".into());

    let quota = TenantQuota::builder()
        .tenant_id("tenant-1".to_string())
        .max_cpu_cores(32)
        .max_memory_mb(65536)
        .max_gpus(Some(8))
        .max_concurrent_jobs(100)
        .max_daily_cost(Some(500.0))
        .max_monthly_cost(Some(15000.0))
        .allowed_pools(allowed_pools)
        .build()
        .expect("Failed to create TenantQuota");

    assert_eq!(quota.tenant_id, "tenant-1");
    assert_eq!(quota.max_cpu_cores, 32);
    assert_eq!(quota.max_memory_mb, 65536);
    assert_eq!(quota.max_gpus, Some(8));
    assert_eq!(quota.max_concurrent_jobs, 100);
    assert_eq!(quota.max_daily_cost, Some(500.0));
    assert_eq!(quota.allowed_pools.len(), 2);
}

#[test]
fn test_tenant_quota_check_within_limits() {
    let quota = create_test_tenant_quota();
    let mut request = create_test_resource_request();

    // This should be within limits
    request.cpu_millicores = 2000;
    request.memory_mb = 4096;
    request.gpu_count = Some(1);

    let validation = quota.check_within_limits(&request);
    assert!(validation.is_ok());
}

#[test]
fn test_tenant_quota_check_exceeds_cpu() {
    let quota = create_test_tenant_quota();
    let mut request = create_test_resource_request();

    // This exceeds CPU limit (16 cores = 16000m)
    request.cpu_millicores = 20000;

    let validation = quota.check_within_limits(&request);
    assert!(validation.is_err());
    assert!(validation.unwrap_err().contains("CPU"));
}

#[test]
fn test_tenant_quota_check_exceeds_memory() {
    let quota = create_test_tenant_quota();
    let mut request = create_test_resource_request();

    // This exceeds memory limit (32GB = 32768MB)
    request.memory_mb = 65536;

    let validation = quota.check_within_limits(&request);
    assert!(validation.is_err());
    let err_msg = validation.unwrap_err();
    assert!(
        err_msg.contains("Memory") || err_msg.contains("memory"),
        "Expected error to contain 'Memory' or 'memory', got: {}",
        err_msg
    );
}

#[test]
fn test_tenant_quota_check_exceeds_gpu() {
    let quota = create_test_tenant_quota();
    let mut request = create_test_resource_request();

    // This exceeds GPU limit (8 GPUs)
    request.gpu_count = Some(10);

    let validation = quota.check_within_limits(&request);
    assert!(validation.is_err());
    assert!(validation.unwrap_err().contains("GPU"));
}

#[test]
fn test_tenant_quota_pool_access_control() {
    let quota = create_test_tenant_quota();
    let pool_id: PoolId = "pool-3".into();

    // pool-3 is not in allowed_pools
    let validation = quota.check_pool_access(&pool_id);
    assert!(validation.is_err());
    assert!(validation.unwrap_err().contains("not allowed"));

    let allowed_pool: PoolId = "pool-1".into();
    let validation = quota.check_pool_access(&allowed_pool);
    assert!(validation.is_ok());
}

#[test]
fn test_pool_status_transitions() {
    let mut pool = create_test_pool();

    // Initial state is Active
    assert_eq!(pool.status, PoolStatus::Active);

    // Can pause pool
    pool.pause();
    assert_eq!(pool.status, PoolStatus::Paused);

    // Can resume pool
    pool.resume();
    assert_eq!(pool.status, PoolStatus::Active);

    // Can drain pool
    pool.drain();
    assert_eq!(pool.status, PoolStatus::Draining);

    // Can mark offline
    pool.mark_offline();
    assert_eq!(pool.status, PoolStatus::Offline);
}

#[test]
fn test_compute_pool_cost_calculation() {
    let mut pool = create_test_pool();
    pool.cost_config = Some(CostConfig {
        cpu_hour_cents: 5,        // $0.05 per CPU core hour
        memory_gb_hour_cents: 2,  // $0.02 per GB memory hour
        gpu_hour_cents: 100,      // $1.00 per GPU hour
        storage_gb_hour_cents: 1, // $0.01 per GB storage hour
        base_hourly_cents: 50,    // $0.50 base hourly cost
    });

    let cost = pool.calculate_cost_per_hour(&PoolCapacity {
        cpu_millicores: 4000,
        memory_mb: 8192,
        gpu_count: 2,
        max_workers: 100,
        active_workers: 20,
        storage_gb: Some(500),
    });

    assert!(cost.is_some());
    let cost_value = cost.unwrap();
    assert!(cost_value > 0.0);

    // CPU cost: 4 cores * $0.05 = $0.20
    // Memory cost: 8 GB * $0.02 = $0.16
    // GPU cost: 2 GPUs * $1.00 = $2.00
    // Storage cost: 500 GB * $0.01 = $5.00
    // Base cost: $0.50
    // Total: $7.86
    assert!((cost_value - 7.86).abs() < 0.01);
}

#[test]
fn test_compute_pool_filtering() {
    let pools = vec![
        create_pool_with_labels("pool-1", &[("env", "prod"), ("region", "us-east")]),
        create_pool_with_labels("pool-2", &[("env", "dev"), ("region", "us-east")]),
        create_pool_with_labels("pool-3", &[("env", "prod"), ("region", "us-west")]),
    ];

    // Filter by env=prod
    let filtered: Vec<_> = pools
        .iter()
        .filter(|p| p.has_label("env", "prod"))
        .collect();
    assert_eq!(filtered.len(), 2);

    // Filter by region=us-east
    let filtered: Vec<_> = pools
        .iter()
        .filter(|p| p.has_label("region", "us-east"))
        .collect();
    assert_eq!(filtered.len(), 2);
}

fn create_test_pool() -> ComputePool {
    ComputePool::builder()
        .id("test-pool".into())
        .name("Test Pool".to_string())
        .provider_type(ProviderType::Kubernetes)
        .total_capacity(PoolCapacity {
            cpu_millicores: 16000,
            memory_mb: 32768,
            gpu_count: 4,
            max_workers: 100,
            active_workers: 0,
            storage_gb: Some(1000),
        })
        .build()
        .expect("Failed to create test pool")
}

fn create_test_tenant_quota() -> TenantQuota {
    TenantQuota::builder()
        .tenant_id("test-tenant".to_string())
        .max_cpu_cores(16)
        .max_memory_mb(32768)
        .max_gpus(Some(8))
        .max_concurrent_jobs(50)
        .max_daily_cost(Some(200.0))
        .allowed_pools(vec!["pool-1".into(), "pool-2".into()])
        .build()
        .expect("Failed to create test quota")
}

fn create_test_resource_request() -> ResourceRequest {
    ResourceRequest::builder()
        .request_id("test-req".into())
        .cpu_millicores(1000)
        .memory_mb(2048)
        .gpu_count(None)
        .tenant_id("test-tenant".to_string())
        .priority(5)
        .build()
        .expect("Failed to create test request")
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
        .expect("Failed to create pool")
}
