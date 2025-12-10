//! Story 8: Implement RBAC Management UI (US-API-ALIGN-008)
//!
//! Tests to validate RBAC Management APIs (US-019) are properly
//! documented, typed, and aligned with frontend expectations.

use chrono::Utc;

#[cfg(test)]
mod tests {
    use super::*;

    use hodei_server::api_docs::{ApiDoc, *};
    use hodei_server::rbac::{
        AccessDecision, AuthToken, CheckPermissionRequest, LoginRequest, Permission, ResourceType,
        RevokeRoleRequest, Role, RoleAssignment, Session, User,
    };
    use std::collections::HashMap;
    use utoipa::OpenApi;

    /// Test 1: Validate Role schema in OpenAPI
    #[tokio::test]
    async fn test_role_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components;
        assert!(components.is_some(), "OpenAPI should have components");

        let schemas = components.unwrap().schemas;
        assert!(
            schemas.contains_key("Role"),
            "Role should be in OpenAPI schemas"
        );

        println!("✅ Role schema defined in OpenAPI");
    }

    /// Test 2: Validate Permission schema in OpenAPI
    #[tokio::test]
    async fn test_permission_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("Permission"),
            "Permission should be in OpenAPI schemas"
        );

        println!("✅ Permission schema defined in OpenAPI");
    }

    /// Test 3: Validate ResourceType schema in OpenAPI
    #[tokio::test]
    async fn test_resource_type_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("ResourceType"),
            "ResourceType should be in OpenAPI schemas"
        );

        println!("✅ ResourceType schema defined in OpenAPI");
    }

    /// Test 4: Validate User schema in OpenAPI
    #[tokio::test]
    async fn test_user_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("User"),
            "User should be in OpenAPI schemas"
        );

        println!("✅ User schema defined in OpenAPI");
    }

    /// Test 5: Validate RoleAssignment schema in OpenAPI
    #[tokio::test]
    async fn test_role_assignment_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("RoleAssignment"),
            "RoleAssignment should be in OpenAPI schemas"
        );

        println!("✅ RoleAssignment schema defined in OpenAPI");
    }

    /// Test 6: Validate PermissionGrant schema in OpenAPI
    #[tokio::test]
    async fn test_permission_grant_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("PermissionGrant"),
            "PermissionGrant should be in OpenAPI schemas"
        );

        println!("✅ PermissionGrant schema defined in OpenAPI");
    }

    /// Test 7: Validate AccessDecision schema in OpenAPI
    #[tokio::test]
    async fn test_access_decision_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("AccessDecision"),
            "AccessDecision should be in OpenAPI schemas"
        );

        println!("✅ AccessDecision schema defined in OpenAPI");
    }

    /// Test 8: Validate AuthToken schema in OpenAPI
    #[tokio::test]
    async fn test_auth_token_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("AuthToken"),
            "AuthToken should be in OpenAPI schemas"
        );

        println!("✅ AuthToken schema defined in OpenAPI");
    }

    /// Test 9: Validate Session schema in OpenAPI
    #[tokio::test]
    async fn test_session_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("Session"),
            "Session should be in OpenAPI schemas"
        );

        println!("✅ Session schema defined in OpenAPI");
    }

    /// Test 10: Validate LoginRequest schema in OpenAPI
    #[tokio::test]
    async fn test_login_request_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("LoginRequest"),
            "LoginRequest should be in OpenAPI schemas"
        );

        println!("✅ LoginRequest schema defined in OpenAPI");
    }

    /// Test 11: Validate RevokeRoleRequest schema in OpenAPI
    #[tokio::test]
    async fn test_revoke_role_request_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("RevokeRoleRequest"),
            "RevokeRoleRequest should be in OpenAPI schemas"
        );

        println!("✅ RevokeRoleRequest schema defined in OpenAPI");
    }

    /// Test 12: Validate CheckPermissionRequest schema in OpenAPI
    #[tokio::test]
    async fn test_check_permission_request_schema_valid() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let components = openapi.components.expect("OpenAPI should have components");
        let schemas = components.schemas;

        assert!(
            schemas.contains_key("CheckPermissionRequest"),
            "CheckPermissionRequest should be in OpenAPI schemas"
        );

        println!("✅ CheckPermissionRequest schema defined in OpenAPI");
    }

    /// Test 13: Test user serialization/deserialization
    #[tokio::test]
    async fn test_user_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("department".to_string(), "Engineering".to_string());
        metadata.insert("title".to_string(), "Senior Developer".to_string());

        let user = User {
            id: "user-123".to_string(),
            username: "john.doe".to_string(),
            email: "john.doe@example.com".to_string(),
            display_name: "John Doe".to_string(),
            is_active: true,
            tenant_id: "tenant-456".to_string(),
            roles: vec![Role::Developer, Role::Manager],
            permissions: vec![Permission::Read, Permission::Write],
            created_at: Utc::now(),
            last_login: Some(Utc::now() - chrono::Duration::hours(2)),
            metadata,
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("id"), "Should contain id");
        assert!(json.contains("username"), "Should contain username");
        assert!(json.contains("email"), "Should contain email");
        assert!(json.contains("roles"), "Should contain roles");

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "user-123");
        assert_eq!(deserialized.username, "john.doe");
        assert_eq!(deserialized.roles.len(), 2);

        println!("✅ User serialization/deserialization works correctly");
    }

    /// Test 14: Test role assignment serialization/deserialization
    #[tokio::test]
    async fn test_role_assignment_serialization() {
        let assignment = RoleAssignment {
            id: "assignment-789".to_string(),
            user_id: "user-123".to_string(),
            role: Role::Manager,
            resource_id: Some("project-456".to_string()),
            resource_type: Some(ResourceType::Pipeline),
            tenant_id: "tenant-456".to_string(),
            granted_by: "admin-001".to_string(),
            granted_at: Utc::now() - chrono::Duration::days(30),
            expires_at: Some(Utc::now() + chrono::Duration::days(335)),
        };

        let json = serde_json::to_string(&assignment).unwrap();
        assert!(json.contains("id"), "Should contain id");
        assert!(json.contains("user_id"), "Should contain user_id");
        assert!(json.contains("role"), "Should contain role");
        assert!(json.contains("resource_id"), "Should contain resource_id");

        let deserialized: RoleAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "assignment-789");
        assert_eq!(deserialized.user_id, "user-123");
        assert_eq!(deserialized.role, Role::Manager);
        assert!(deserialized.resource_id.is_some());

        println!("✅ RoleAssignment serialization/deserialization works correctly");
    }

    /// Test 15: Test auth token serialization/deserialization
    #[tokio::test]
    async fn test_auth_token_serialization() {
        let token = AuthToken {
            id: "token-123".to_string(),
            user_id: "user-123".to_string(),
            token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            scopes: vec![Permission::Read, Permission::Write, Permission::Execute],
            created_at: Utc::now(),
            last_used: Some(Utc::now() - chrono::Duration::minutes(5)),
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("id"), "Should contain id");
        assert!(json.contains("user_id"), "Should contain user_id");
        assert!(json.contains("token"), "Should contain token");
        assert!(json.contains("scopes"), "Should contain scopes");

        let deserialized: AuthToken = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "token-123");
        assert_eq!(deserialized.user_id, "user-123");
        assert_eq!(deserialized.scopes.len(), 3);

        println!("✅ AuthToken serialization/deserialization works correctly");
    }

    /// Test 16: Test access decision serialization/deserialization
    #[tokio::test]
    async fn test_access_decision_serialization() {
        let decision = AccessDecision {
            allowed: true,
            permission: Permission::Write,
            resource_type: ResourceType::Pipeline,
            resource_id: Some("pipeline-123".to_string()),
            reason: "User has Manager role with write permissions".to_string(),
            effective_permissions: vec![Permission::Read, Permission::Write, Permission::Execute],
        };

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("allowed"), "Should contain allowed");
        assert!(json.contains("permission"), "Should contain permission");
        assert!(json.contains("reason"), "Should contain reason");
        assert!(json.contains("effective_permissions"), "Should contain effective_permissions");

        let deserialized: AccessDecision = serde_json::from_str(&json).unwrap();
        assert!(deserialized.allowed);
        assert_eq!(deserialized.permission, Permission::Write);
        assert_eq!(deserialized.resource_type, ResourceType::Pipeline);
        assert_eq!(deserialized.effective_permissions.len(), 3);

        println!("✅ AccessDecision serialization/deserialization works correctly");
    }

    /// Test 17: Test session serialization/deserialization
    #[tokio::test]
    async fn test_session_serialization() {
        let session = Session {
            id: "session-456".to_string(),
            user_id: "user-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            login_at: Utc::now() - chrono::Duration::hours(8),
            last_activity: Utc::now() - chrono::Duration::minutes(2),
            ip_address: "192.168.1.100".to_string(),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".to_string(),
            is_active: true,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("id"), "Should contain id");
        assert!(json.contains("user_id"), "Should contain user_id");
        assert!(json.contains("ip_address"), "Should contain ip_address");
        assert!(json.contains("is_active"), "Should contain is_active");

        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "session-456");
        assert_eq!(deserialized.user_id, "user-123");
        assert!(deserialized.is_active);

        println!("✅ Session serialization/deserialization works correctly");
    }

    /// Test 18: Test login request serialization/deserialization
    #[tokio::test]
    async fn test_login_request_serialization() {
        let request = LoginRequest {
            username: "admin".to_string(),
            password: "secure_password_123".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("username"), "Should contain username");
        assert!(json.contains("password"), "Should contain password");

        let deserialized: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, "admin");
        assert_eq!(deserialized.password, "secure_password_123");

        println!("✅ LoginRequest serialization/deserialization works correctly");
    }

    /// Test 19: Test revoke role request serialization/deserialization
    #[tokio::test]
    async fn test_revoke_role_request_serialization() {
        let request = RevokeRoleRequest {
            assignment_id: "assignment-789".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("assignment_id"), "Should contain assignment_id");

        let deserialized: RevokeRoleRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.assignment_id, "assignment-789");

        println!("✅ RevokeRoleRequest serialization/deserialization works correctly");
    }

    /// Test 20: Test check permission request serialization/deserialization
    #[tokio::test]
    async fn test_check_permission_request_serialization() {
        let request = CheckPermissionRequest {
            user_id: "user-123".to_string(),
            permission: Permission::Delete,
            resource_type: ResourceType::Worker,
            resource_id: Some("worker-456".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("user_id"), "Should contain user_id");
        assert!(json.contains("permission"), "Should contain permission");
        assert!(json.contains("resource_type"), "Should contain resource_type");
        assert!(json.contains("resource_id"), "Should contain resource_id");

        let deserialized: CheckPermissionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, "user-123");
        assert_eq!(deserialized.permission, Permission::Delete);
        assert_eq!(deserialized.resource_type, ResourceType::Worker);

        println!("✅ CheckPermissionRequest serialization/deserialization works correctly");
    }

    /// Test 21: Test Role enum values
    #[tokio::test]
    async fn test_role_enum_values() {
        let roles = vec![
            Role::SuperAdmin,
            Role::Admin,
            Role::Manager,
            Role::Developer,
            Role::Viewer,
            Role::Guest,
        ];

        let expected_names = vec![
            "SuperAdmin",
            "Admin",
            "Manager",
            "Developer",
            "Viewer",
            "Guest",
        ];

        for (role, expected) in roles.into_iter().zip(expected_names) {
            let json = serde_json::to_string(&role).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }

        println!("✅ Role enum values are correct");
    }

    /// Test 22: Test Permission enum values
    #[tokio::test]
    async fn test_permission_enum_values() {
        let permissions = vec![
            Permission::Read,
            Permission::Write,
            Permission::Delete,
            Permission::Admin,
            Permission::Execute,
            Permission::Grant,
        ];

        let expected_values = vec!["Read", "Write", "Delete", "Admin", "Execute", "Grant"];

        for (permission, expected) in permissions.into_iter().zip(expected_values) {
            let json = serde_json::to_string(&permission).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: Permission = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, permission);
        }

        println!("✅ Permission enum values are correct");
    }

    /// Test 23: Test ResourceType enum values
    #[tokio::test]
    async fn test_resource_type_enum_values() {
        let resource_types = vec![
            ResourceType::Pipeline,
            ResourceType::Execution,
            ResourceType::Worker,
            ResourceType::ResourcePool,
            ResourceType::Metrics,
            ResourceType::Logs,
            ResourceType::Traces,
            ResourceType::Alerts,
            ResourceType::Cost,
            ResourceType::Security,
            ResourceType::Users,
            ResourceType::System,
        ];

        let expected_values = vec![
            "Pipeline",
            "Execution",
            "Worker",
            "ResourcePool",
            "Metrics",
            "Logs",
            "Traces",
            "Alerts",
            "Cost",
            "Security",
            "Users",
            "System",
        ];

        for (resource_type, expected) in resource_types.into_iter().zip(expected_values) {
            let json = serde_json::to_string(&resource_type).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected));

            let deserialized: ResourceType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, resource_type);
        }

        println!("✅ ResourceType enum values are correct");
    }

    /// Test 24: Test RBAC management endpoints documented in OpenAPI
    #[tokio::test]
    async fn test_rbac_endpoints_documented() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let paths = &openapi.paths.paths;

        println!("\n🔐 RBAC Management Endpoints Check");
        println!("======================================");

        // Check for RBAC endpoints
        assert!(
            paths.contains_key("/api/v1/auth/login"),
            "Should have /api/v1/auth/login endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/auth/users"),
            "Should have /api/v1/auth/users endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/auth/users/{id}"),
            "Should have /api/v1/auth/users/{{id}} endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/auth/users/{id}/roles"),
            "Should have /api/v1/auth/users/{{id}}/roles endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/auth/roles/assign"),
            "Should have /api/v1/auth/roles/assign endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/auth/roles/revoke"),
            "Should have /api/v1/auth/roles/revoke endpoint"
        );
        assert!(
            paths.contains_key("/api/v1/auth/check"),
            "Should have /api/v1/auth/check endpoint"
        );

        println!("  ✅ /api/v1/auth/login documented");
        println!("  ✅ /api/v1/auth/users documented");
        println!("  ✅ /api/v1/auth/users/{{id}} documented");
        println!("  ✅ /api/v1/auth/users/{{id}}/roles documented");
        println!("  ✅ /api/v1/auth/roles/assign documented");
        println!("  ✅ /api/v1/auth/roles/revoke documented");
        println!("  ✅ /api/v1/auth/check documented");
        println!("✅ All RBAC management endpoints documented");
    }

    /// Test 25: Test RBAC API OpenAPI completeness
    #[tokio::test]
    async fn test_rbac_openapi_completeness() {
        let openapi = <ApiDoc as OpenApi>::openapi();
        let paths = openapi.paths.paths;

        let mut rbac_count = 0;

        // Count RBAC-related endpoints
        for (path, _) in paths.iter() {
            if path.contains("/auth/") {
                rbac_count += 1;
            }
        }

        println!("\n🔐 RBAC Management API Coverage");
        println!("======================================");
        println!("Total RBAC Management Endpoints: {}", rbac_count);

        // Minimum expected: login, users CRUD, roles, permission check
        assert!(
            rbac_count >= 7,
            "Expected at least 7 RBAC management endpoints, found {}",
            rbac_count
        );

        println!("✅ RBAC Management API coverage test passed");
    }
}
