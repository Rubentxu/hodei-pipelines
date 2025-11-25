//! End-to-end integration test for tenant management
//!
//! This test verifies that the tenant management API endpoints
//! are properly integrated and functional.

use crate::tenant_management::{CreateTenantRequest, TenantManagementService, UpdateTenantRequest};
use hodei_modules::multi_tenancy_quota_manager::MultiTenancyQuotaManager;
use std::sync::Arc;

#[tokio::test]
async fn test_end_to_end_tenant_lifecycle() {
    // Setup: Create tenant management service
    let quota_manager_config =
        hodei_modules::multi_tenancy_quota_manager::QuotaManagerConfig::default();
    let quota_manager = Arc::new(MultiTenancyQuotaManager::new(quota_manager_config));
    let service = TenantManagementService::new(quota_manager);

    // Test 1: Create a tenant
    let create_request = CreateTenantRequest {
        name: "test-tenant-e2e".to_string(),
        email: "test@example.com".to_string(),
    };

    let created_tenant = service.create_tenant(create_request).await.unwrap();
    println!("Created tenant: {:?}", created_tenant);
    assert_eq!(created_tenant.name, "test-tenant-e2e");
    assert_eq!(created_tenant.email, "test@example.com");

    // Test 2: Retrieve the tenant
    let retrieved_tenant = service.get_tenant(&created_tenant.id).await.unwrap();
    println!("Retrieved tenant: {:?}", retrieved_tenant);
    assert_eq!(retrieved_tenant.id, created_tenant.id);
    assert_eq!(retrieved_tenant.name, created_tenant.name);

    // Test 3: Update the tenant
    let update_request = UpdateTenantRequest {
        name: "updated-tenant-e2e".to_string(),
        email: "updated@example.com".to_string(),
    };

    let updated_tenant = service
        .update_tenant(&created_tenant.id, update_request)
        .await
        .unwrap();
    println!("Updated tenant: {:?}", updated_tenant);
    assert_eq!(updated_tenant.name, "updated-tenant-e2e");
    assert_eq!(updated_tenant.email, "updated@example.com");

    // Test 4: Get quota for the tenant
    let quota = service.get_quota(&created_tenant.id).await.unwrap();
    println!("Tenant quota: {:?}", quota);
    assert!(quota.cpu_m > 0);
    assert!(quota.memory_mb > 0);
    assert!(quota.max_concurrent_jobs > 0);

    // Test 5: Update quota for the tenant
    use chrono::Utc;
    use hodei_modules::multi_tenancy_quota_manager::{
        BillingTier, BurstPolicy, QuotaLimits, QuotaType,
    };
    use std::collections::HashMap;

    let new_quota = hodei_modules::multi_tenancy_quota_manager::TenantQuota {
        tenant_id: created_tenant.id.clone(),
        limits: QuotaLimits {
            max_cpu_cores: 8000,
            max_memory_mb: 16384,
            max_concurrent_workers: 40,
            max_concurrent_jobs: 20,
            max_daily_cost: 500.0,
            max_monthly_jobs: 5000,
        },
        pool_access: HashMap::new(),
        burst_policy: BurstPolicy {
            allowed: true,
            max_burst_multiplier: 2.0,
            burst_duration: std::time::Duration::from_secs(900),
            cooldown_period: std::time::Duration::from_secs(1800),
            max_bursts_per_day: 30,
        },
        billing_tier: BillingTier::Premium,
        quota_type: QuotaType::SoftLimit,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let updated_quota = service
        .update_quota(&created_tenant.id, new_quota)
        .await
        .unwrap();
    println!("Updated quota: {:?}", updated_quota);
    assert_eq!(updated_quota.cpu_m, 8000);
    assert_eq!(updated_quota.memory_mb, 16384);
    assert_eq!(updated_quota.max_concurrent_jobs, 20);

    // Test 6: Delete the tenant
    service.delete_tenant(&created_tenant.id).await.unwrap();

    // Verify tenant is deleted
    let result = service.get_tenant(&created_tenant.id).await;
    assert!(result.is_err());
}
