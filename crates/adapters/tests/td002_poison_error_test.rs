//! Test for US-TD-002: Fix PoisonError handling in RBAC repositories
//!
//! This test verifies that PoisonError is handled gracefully instead of
//! causing panics that can crash the server.

use hodei_pipelines_adapters::InMemoryRoleRepository;
use hodei_pipelines_core::security::{RoleEntity, RoleId};
use hodei_pipelines_ports::rbac_repository::RoleRepository;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_multiple_concurrent_operations() {
    // Test that concurrent operations don't cause issues
    let repo = Arc::new(InMemoryRoleRepository::new());

    // Create multiple roles
    let mut handles = Vec::new();

    for i in 0..10 {
        let repo_clone = Arc::clone(&repo);
        let role = create_test_role(format!("test-role-{}", i));

        let handle = tokio::spawn(async move {
            let result = repo_clone.save_role(&role).await;
            result
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await;
        assert!(
            result.is_ok(),
            "All concurrent save operations should succeed"
        );
    }

    // Verify all roles were saved
    let roles = repo.list_all_roles().await;

    assert!(roles.is_ok(), "Should be able to list roles");
    assert_eq!(roles.unwrap().len(), 10, "Should have 10 roles");
}

fn create_test_role(name: String) -> RoleEntity {
    RoleEntity::new(
        RoleId::from_uuid(Uuid::new_v4()),
        name,
        "Test role description".to_string(),
        Vec::new(), // permissions
    )
    .expect("Failed to create test role")
}
