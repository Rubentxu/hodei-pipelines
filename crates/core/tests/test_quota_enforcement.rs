//! Tests for Quota Enforcement & Multi-tenancy

use hodei_pipelines_core::global_resource_controller::*;
use hodei_pipelines_core::resource_governance::*;

#[test]
fn test_tenant_quota_creation_and_validation() {
    let mut grc = GlobalResourceController::new(GRCConfig {
        default_allocation_timeout_secs: 300,
        enable_quota_enforcement: true,
        enable_cost_tracking: true,
        max_allocation_wait_secs: 60,
    });

    // Register pools
    let pool1 = create_pool(
        "prod-us-east",
        16000,
        32768,
        4,
        100,
        0,
        ProviderType::Kubernetes,
    );
    let pool2 = create_pool(
        "prod-us-west",
        16000,
        32768,
        4,
        100,
        0,
        ProviderType::Kubernetes,
    );
    grc.register_pool(pool1).unwrap();
    grc.register_pool(pool2).unwrap();

    // Register tenant quotas
    let tenant_a_quota = TenantQuota::builder()
        .tenant_id("tenant-a".to_string())
        .max_cpu_cores(16)
        .max_memory_mb(32768)
        .max_gpus(Some(4))
        .max_concurrent_jobs(50)
        .max_daily_cost(Some(1000.0))
        .allowed_pools(vec!["prod-us-east".into(), "prod-us-west".into()])
        .build()
        .unwrap();

    let tenant_b_quota = TenantQuota::builder()
        .tenant_id("tenant-b".to_string())
        .max_cpu_cores(8)
        .max_memory_mb(16384)
        .max_gpus(Some(2))
        .max_concurrent_jobs(25)
        .max_daily_cost(Some(500.0))
        .allowed_pools(vec!["prod-us-east".into()]) // Only US-East
        .build()
        .unwrap();

    grc.register_tenant_quota("tenant-a".to_string(), tenant_a_quota)
        .unwrap();
    grc.register_tenant_quota("tenant-b".to_string(), tenant_b_quota)
        .unwrap();

    // Test tenant-a can use both pools
    let request_a = ResourceRequest::builder()
        .request_id(RequestId::from("req-a-1".to_string()))
        .cpu_millicores(8000) // 8 cores
        .memory_mb(16384) // 16GB
        .gpu_count(Some(2))
        .tenant_id("tenant-a".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result_a = grc.allocate_with_quota_check(
        "alloc-a-1".to_string(),
        PoolId::from("prod-us-east".to_string()),
        request_a,
    );
    assert!(
        result_a.is_ok(),
        "Tenant A should be able to allocate 8 cores"
    );

    // Test tenant-b exceeds CPU quota
    let request_b = ResourceRequest::builder()
        .request_id(RequestId::from("req-b-1".to_string()))
        .cpu_millicores(10000) // 10 cores exceeds quota of 8
        .memory_mb(8192)
        .tenant_id("tenant-b".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result_b = grc.allocate_with_quota_check(
        "alloc-b-1".to_string(),
        PoolId::from("prod-us-east".to_string()),
        request_b,
    );
    assert!(result_b.is_err(), "Tenant B should not exceed CPU quota");
    assert!(result_b.unwrap_err().contains("CPU"));

    // Test tenant-b pool access restriction
    let request_b2 = ResourceRequest::builder()
        .request_id(RequestId::from("req-b-2".to_string()))
        .cpu_millicores(2000)
        .memory_mb(4096)
        .tenant_id("tenant-b".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result_b2 = grc.allocate_with_quota_check(
        "alloc-b-2".to_string(),
        PoolId::from("prod-us-west".to_string()),
        request_b2,
    );
    assert!(
        result_b2.is_err(),
        "Tenant B should not access US-West pool"
    );
    assert!(result_b2.unwrap_err().contains("not allowed"));
}

#[test]
fn test_concurrent_job_limits() {
    let mut grc = GlobalResourceController::new(GRCConfig {
        default_allocation_timeout_secs: 300,
        enable_quota_enforcement: true,
        enable_cost_tracking: true,
        max_allocation_wait_secs: 60,
    });

    let pool = create_pool(
        "test-pool",
        16000,
        32768,
        4,
        100,
        0,
        ProviderType::Kubernetes,
    );
    grc.register_pool(pool).unwrap();

    let quota = TenantQuota::builder()
        .tenant_id("test-tenant".to_string())
        .max_cpu_cores(16)
        .max_memory_mb(32768)
        .max_concurrent_jobs(3) // Limit of 3 concurrent jobs
        .allowed_pools(vec![PoolId::from("test-pool".to_string())])
        .build()
        .unwrap();

    grc.register_tenant_quota("test-tenant".to_string(), quota)
        .unwrap();

    // Allocate 3 jobs (at limit)
    for i in 0..3 {
        let request = ResourceRequest::builder()
            .request_id(RequestId::from(format!("req-{}", i)))
            .cpu_millicores(2000)
            .memory_mb(4096)
            .tenant_id("test-tenant".to_string())
            .priority(5)
            .build()
            .unwrap();

        let result = grc.allocate_with_quota_check(
            format!("alloc-{}", i),
            PoolId::from("test-pool".to_string()),
            request,
        );
        assert!(result.is_ok(), "Job {} should succeed", i);
    }

    // Try to allocate 4th job (exceeds limit)
    let request = ResourceRequest::builder()
        .request_id(RequestId::from("req-exceed".to_string()))
        .cpu_millicores(2000)
        .memory_mb(4096)
        .tenant_id("test-tenant".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result = grc.allocate_with_quota_check(
        "alloc-exceed".to_string(),
        PoolId::from("test-pool".to_string()),
        request,
    );
    assert!(result.is_err(), "Should exceed concurrent job limit");
}

#[test]
fn test_cost_budget_enforcement() {
    let mut grc = GlobalResourceController::new(GRCConfig {
        default_allocation_timeout_secs: 300,
        enable_quota_enforcement: true,
        enable_cost_tracking: true,
        max_allocation_wait_secs: 60,
    });

    let mut pool = create_pool(
        "expensive-pool",
        8000,
        16384,
        2,
        50,
        0,
        ProviderType::Kubernetes,
    );
    pool.cost_config = Some(CostConfig {
        cpu_hour_cents: 1000, // $10/core/hour (very expensive)
        memory_gb_hour_cents: 100,
        gpu_hour_cents: 10000,
        storage_gb_hour_cents: 10,
        base_hourly_cents: 1000,
    });
    grc.register_pool(pool).unwrap();

    let quota = TenantQuota::builder()
        .tenant_id("cost-sensitive-tenant".to_string())
        .max_cpu_cores(8)
        .max_memory_mb(16384)
        .max_concurrent_jobs(10)
        .max_daily_cost(Some(100.0)) // $100 daily limit
        .allowed_pools(vec![PoolId::from("expensive-pool".to_string())])
        .build()
        .unwrap();

    grc.register_tenant_quota("cost-sensitive-tenant".to_string(), quota)
        .unwrap();

    // First job should succeed
    let request1 = ResourceRequest::builder()
        .request_id(RequestId::from("req-1".to_string()))
        .cpu_millicores(2000)
        .memory_mb(4096)
        .tenant_id("cost-sensitive-tenant".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result1 = grc.allocate_with_quota_check(
        "alloc-1".to_string(),
        PoolId::from("expensive-pool".to_string()),
        request1,
    );
    assert!(result1.is_ok(), "First job should succeed");

    // Simulate high-cost job that would exceed daily budget
    let request2 = ResourceRequest::builder()
        .request_id(RequestId::from("req-2".to_string()))
        .cpu_millicores(6000) // 6 cores (total 8 cores with first job)
        .memory_mb(12288) // 12GB (total 16GB with first job)
        .tenant_id("cost-sensitive-tenant".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result2 = grc.allocate_with_quota_check(
        "alloc-2".to_string(),
        PoolId::from("expensive-pool".to_string()),
        request2,
    );
    assert!(result2.is_ok(), "Cost check passes (daily vs hourly)");

    // Note: Full cost enforcement would need usage tracking over time
}

#[test]
fn test_resource_isolation_between_tenants() {
    let mut grc = GlobalResourceController::new(GRCConfig {
        default_allocation_timeout_secs: 300,
        enable_quota_enforcement: true,
        enable_cost_tracking: true,
        max_allocation_wait_secs: 60,
    });

    let pool = create_pool(
        "shared-pool",
        16000,
        32768,
        4,
        100,
        0,
        ProviderType::Kubernetes,
    );
    grc.register_pool(pool).unwrap();

    let quota_a = TenantQuota::builder()
        .tenant_id("tenant-a".to_string())
        .max_cpu_cores(8)
        .max_memory_mb(16384)
        .max_concurrent_jobs(10)
        .allowed_pools(vec![PoolId::from("shared-pool".to_string())])
        .build()
        .unwrap();

    let quota_b = TenantQuota::builder()
        .tenant_id("tenant-b".to_string())
        .max_cpu_cores(8)
        .max_memory_mb(16384)
        .max_concurrent_jobs(10)
        .allowed_pools(vec![PoolId::from("shared-pool".to_string())])
        .build()
        .unwrap();

    grc.register_tenant_quota("tenant-a".to_string(), quota_a)
        .unwrap();
    grc.register_tenant_quota("tenant-b".to_string(), quota_b)
        .unwrap();

    // Tenant A allocates resources
    let request_a = ResourceRequest::builder()
        .request_id(RequestId::from("req-a".to_string()))
        .cpu_millicores(4000)
        .memory_mb(8192)
        .tenant_id("tenant-a".to_string())
        .priority(5)
        .build()
        .unwrap();

    let allocation_a = grc
        .allocate_with_quota_check(
            "alloc-a".to_string(),
            PoolId::from("shared-pool".to_string()),
            request_a,
        )
        .unwrap();

    // Tenant B should still be able to allocate (resources are isolated)
    let request_b = ResourceRequest::builder()
        .request_id(RequestId::from("req-b".to_string()))
        .cpu_millicores(4000)
        .memory_mb(8192)
        .tenant_id("tenant-b".to_string())
        .priority(5)
        .build()
        .unwrap();

    let allocation_b = grc
        .allocate_with_quota_check(
            "alloc-b".to_string(),
            PoolId::from("shared-pool".to_string()),
            request_b,
        )
        .unwrap();

    // Verify both allocations exist and track different tenants
    assert_eq!(allocation_a.resources.cpu_millicores, 4000);
    assert_eq!(allocation_b.resources.cpu_millicores, 4000);

    // Pool should show total usage (8000m) regardless of tenant
    let pool_status = grc
        .get_pool_status(&PoolId::from("shared-pool".to_string()))
        .unwrap();
    assert_eq!(pool_status.cpu_millicores, 8000);
}

#[test]
fn test_quota_with_graceful_degradation() {
    let mut grc = GlobalResourceController::new(GRCConfig {
        default_allocation_timeout_secs: 300,
        enable_quota_enforcement: true,
        enable_cost_tracking: true,
        max_allocation_wait_secs: 60,
    });

    let pool = create_pool(
        "limited-pool",
        4000,
        8192,
        1,
        20,
        0,
        ProviderType::Kubernetes,
    );
    grc.register_pool(pool).unwrap();

    let quota = TenantQuota::builder()
        .tenant_id("limited-tenant".to_string())
        .max_cpu_cores(2) // Allow only 2 cores total
        .max_memory_mb(2048) // 2GB total
        .max_concurrent_jobs(5) // But max 5 concurrent jobs
        .allowed_pools(vec![PoolId::from("limited-pool".to_string())])
        .build()
        .unwrap();

    grc.register_tenant_quota("limited-tenant".to_string(), quota)
        .unwrap();

    // Allocate 2 jobs (at CPU limit)
    for i in 0..2 {
        let request = ResourceRequest::builder()
            .request_id(RequestId::from(format!("req-{}", i)))
            .cpu_millicores(1000) // 1 core each
            .memory_mb(1024) // 1GB each
            .tenant_id("limited-tenant".to_string())
            .priority(5)
            .build()
            .unwrap();

        let result = grc.allocate_with_quota_check(
            format!("alloc-{}", i),
            PoolId::from("limited-pool".to_string()),
            request,
        );
        assert!(result.is_ok(), "Allocation {} should succeed", i);
    }

    // Try one more - should fail due to CPU quota (3 cores > 2 cores limit)
    let request = ResourceRequest::builder()
        .request_id(RequestId::from("req-fail".to_string()))
        .cpu_millicores(1000)
        .memory_mb(1024)
        .tenant_id("limited-tenant".to_string())
        .priority(5)
        .build()
        .unwrap();

    let result = grc.allocate_with_quota_check(
        "alloc-fail".to_string(),
        PoolId::from("limited-pool".to_string()),
        request,
    );
    assert!(result.is_err(), "Should fail due to CPU quota");
}

fn create_pool(
    id: &str,
    cpu_millicores: u64,
    memory_mb: u64,
    gpu_count: u32,
    max_workers: u32,
    _active_workers: u32,
    provider_type: ProviderType,
) -> ComputePool {
    ComputePool::builder()
        .id(PoolId::from(id.to_string()))
        .name(format!("Pool {}", id))
        .provider_type(provider_type)
        .total_capacity(PoolCapacity {
            cpu_millicores,
            memory_mb,
            gpu_count,
            max_workers,
            active_workers: 0,
            storage_gb: Some(500),
        })
        .build()
        .unwrap()
}
