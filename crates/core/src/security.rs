use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DomainError;

// ========== Value Objects for RBAC ==========

/// Role identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(pub Uuid);

impl RoleId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for RoleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RoleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for RoleId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Permission identifier - Value Object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionId(pub Uuid);

impl PermissionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for PermissionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PermissionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PermissionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

// ========== RBAC Domain Models ==========

/// Role entity with permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleEntity {
    pub id: RoleId,
    pub name: String,
    pub description: String,
    pub permissions: Vec<PermissionId>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RoleEntity {
    pub fn new(
        id: RoleId,
        name: String,
        description: String,
        permissions: Vec<PermissionId>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Role name cannot be empty".to_string(),
            ));
        }
        if name.len() > 255 {
            return Err(DomainError::Validation("Role name too long".to_string()));
        }
        if description.len() > 1000 {
            return Err(DomainError::Validation(
                "Role description too long".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        Ok(Self {
            id,
            name,
            description,
            permissions,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<(), DomainError> {
        if let Some(ref new_name) = name {
            if new_name.trim().is_empty() {
                return Err(DomainError::Validation(
                    "Role name cannot be empty".to_string(),
                ));
            }
            if new_name.len() > 255 {
                return Err(DomainError::Validation("Role name too long".to_string()));
            }
            self.name = new_name.clone();
        }

        if let Some(ref new_description) = description {
            if new_description.len() > 1000 {
                return Err(DomainError::Validation(
                    "Role description too long".to_string(),
                ));
            }
            self.description = new_description.clone();
        }

        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    pub fn add_permission(&mut self, permission: PermissionId) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
            self.updated_at = chrono::Utc::now();
        }
    }

    pub fn remove_permission(&mut self, permission: &PermissionId) {
        self.permissions.retain(|p| p != permission);
        self.updated_at = chrono::Utc::now();
    }

    pub fn has_permission(&self, permission: &PermissionId) -> bool {
        self.permissions.contains(permission)
    }
}

/// Permission entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionEntity {
    pub id: PermissionId,
    pub name: String,
    pub description: String,
    pub resource: String,
    pub action: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PermissionEntity {
    pub fn new(
        id: PermissionId,
        name: String,
        description: String,
        resource: String,
        action: String,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Permission name cannot be empty".to_string(),
            ));
        }
        if name.len() > 255 {
            return Err(DomainError::Validation(
                "Permission name too long".to_string(),
            ));
        }
        if resource.trim().is_empty() {
            return Err(DomainError::Validation(
                "Permission resource cannot be empty".to_string(),
            ));
        }
        if action.trim().is_empty() {
            return Err(DomainError::Validation(
                "Permission action cannot be empty".to_string(),
            ));
        }

        let now = chrono::Utc::now();
        Ok(Self {
            id,
            name,
            description,
            resource,
            action,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update(&mut self, description: Option<String>) {
        if let Some(ref new_description) = description {
            self.description = new_description.clone();
        }
        self.updated_at = chrono::Utc::now();
    }

    /// Get permission key in format "resource:action"
    pub fn key(&self) -> String {
        format!("{}:{}", self.resource, self.action)
    }
}

/// Legacy Role enum (for backward compatibility)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Operator,
    Viewer,
    Worker,
    System,
}

/// Legacy Permission enum (for backward compatibility)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ReadJobs,
    WriteJobs,
    DeleteJobs,
    ManageWorkers,
    ViewMetrics,
    AdminSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String, // Subject (User ID or Worker ID)
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub subject: String,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
    pub tenant_id: Option<String>,
}

impl SecurityContext {
    pub fn new(
        subject: String,
        roles: Vec<Role>,
        permissions: Vec<Permission>,
        tenant_id: Option<String>,
    ) -> Self {
        Self {
            subject,
            roles,
            permissions,
            tenant_id,
        }
    }

    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin) || self.has_role(&Role::System)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_admin_context() -> SecurityContext {
        SecurityContext {
            subject: "admin-user".to_string(),
            roles: vec![Role::Admin],
            permissions: vec![
                Permission::ReadJobs,
                Permission::WriteJobs,
                Permission::DeleteJobs,
                Permission::ManageWorkers,
                Permission::ViewMetrics,
                Permission::AdminSystem,
            ],
            tenant_id: Some("admin-tenant".to_string()),
        }
    }

    fn create_worker_context() -> SecurityContext {
        SecurityContext {
            subject: "worker-user".to_string(),
            roles: vec![Role::Worker],
            permissions: vec![Permission::ReadJobs, Permission::WriteJobs],
            tenant_id: Some("worker-tenant".to_string()),
        }
    }

    fn create_viewer_context() -> SecurityContext {
        SecurityContext {
            subject: "viewer-user".to_string(),
            roles: vec![Role::Viewer],
            permissions: vec![Permission::ReadJobs, Permission::ViewMetrics],
            tenant_id: Some("viewer-tenant".to_string()),
        }
    }

    fn create_anonymous_context() -> SecurityContext {
        SecurityContext {
            subject: "anonymous".to_string(),
            roles: vec![],
            permissions: vec![],
            tenant_id: None,
        }
    }

    // Role tests
    #[test]
    fn test_role_enum_variants() {
        assert_eq!(format!("{:?}", Role::Admin), "Admin");
        assert_eq!(format!("{:?}", Role::Operator), "Operator");
        assert_eq!(format!("{:?}", Role::Viewer), "Viewer");
        assert_eq!(format!("{:?}", Role::Worker), "Worker");
        assert_eq!(format!("{:?}", Role::System), "System");
    }

    #[test]
    fn test_role_equality() {
        assert_eq!(Role::Admin, Role::Admin);
        assert_ne!(Role::Admin, Role::Worker);
        assert_ne!(Role::Viewer, Role::Operator);
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::Admin;
        let serialized = serde_json::to_string(&role).unwrap();
        assert_eq!(serialized, "\"Admin\"");

        let deserialized: Role = serde_json::from_str("\"Admin\"").unwrap();
        assert_eq!(deserialized, Role::Admin);
    }

    #[test]
    fn test_role_clone() {
        let role = Role::Worker;
        let cloned = role.clone();
        assert_eq!(role, cloned);
    }

    // Permission tests
    #[test]
    fn test_permission_enum_variants() {
        assert_eq!(format!("{:?}", Permission::ReadJobs), "ReadJobs");
        assert_eq!(format!("{:?}", Permission::WriteJobs), "WriteJobs");
        assert_eq!(format!("{:?}", Permission::DeleteJobs), "DeleteJobs");
        assert_eq!(format!("{:?}", Permission::ManageWorkers), "ManageWorkers");
        assert_eq!(format!("{:?}", Permission::ViewMetrics), "ViewMetrics");
        assert_eq!(format!("{:?}", Permission::AdminSystem), "AdminSystem");
    }

    #[test]
    fn test_permission_equality() {
        assert_eq!(Permission::ReadJobs, Permission::ReadJobs);
        assert_ne!(Permission::ReadJobs, Permission::WriteJobs);
        assert_ne!(Permission::DeleteJobs, Permission::ManageWorkers);
    }

    #[test]
    fn test_permission_serialization() {
        let permission = Permission::ReadJobs;
        let serialized = serde_json::to_string(&permission).unwrap();
        assert_eq!(serialized, "\"ReadJobs\"");

        let deserialized: Permission = serde_json::from_str("\"ReadJobs\"").unwrap();
        assert_eq!(deserialized, Permission::ReadJobs);
    }

    #[test]
    fn test_permission_clone() {
        let permission = Permission::WriteJobs;
        let cloned = permission.clone();
        assert_eq!(permission, cloned);
    }

    #[test]
    fn test_all_permissions_are_distinct() {
        let permissions = vec![
            Permission::ReadJobs,
            Permission::WriteJobs,
            Permission::DeleteJobs,
            Permission::ManageWorkers,
            Permission::ViewMetrics,
            Permission::AdminSystem,
        ];

        for (i, perm1) in permissions.iter().enumerate() {
            for (j, perm2) in permissions.iter().enumerate() {
                if i != j {
                    assert_ne!(perm1, perm2);
                }
            }
        }
    }

    // JwtClaims tests
    #[test]
    fn test_jwt_claims_creation() {
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: 1234567890,
            iat: 1234567890,
            roles: vec![Role::Admin],
            permissions: vec![Permission::AdminSystem],
            tenant_id: Some("test-tenant".to_string()),
        };

        assert_eq!(claims.sub, "test-user");
        assert_eq!(claims.exp, 1234567890);
        assert_eq!(claims.iat, 1234567890);
        assert_eq!(claims.roles.len(), 1);
        assert_eq!(claims.permissions.len(), 1);
        assert!(claims.tenant_id.is_some());
    }

    #[test]
    fn test_jwt_claims_with_empty_tenant() {
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: 1234567890,
            iat: 1234567890,
            roles: vec![Role::Worker],
            permissions: vec![Permission::ReadJobs],
            tenant_id: None,
        };

        assert!(claims.tenant_id.is_none());
    }

    #[test]
    fn test_jwt_claims_serialization() {
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: 1234567890,
            iat: 1234567890,
            roles: vec![Role::Admin],
            permissions: vec![Permission::ReadJobs, Permission::WriteJobs],
            tenant_id: Some("test-tenant".to_string()),
        };

        let serialized = serde_json::to_string(&claims).unwrap();
        let deserialized: JwtClaims = serde_json::from_str(&serialized).unwrap();

        assert_eq!(claims.sub, deserialized.sub);
        assert_eq!(claims.exp, deserialized.exp);
        assert_eq!(claims.roles, deserialized.roles);
        assert_eq!(claims.permissions, deserialized.permissions);
        assert_eq!(claims.tenant_id, deserialized.tenant_id);
    }

    #[test]
    fn test_jwt_claims_clone() {
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            exp: 1234567890,
            iat: 1234567890,
            roles: vec![Role::Worker],
            permissions: vec![Permission::ReadJobs],
            tenant_id: Some("test-tenant".to_string()),
        };

        let cloned = claims.clone();
        assert_eq!(claims.sub, cloned.sub);
        assert_eq!(claims.exp, cloned.exp);
        assert_eq!(claims.iat, cloned.iat);
        assert_eq!(claims.roles, cloned.roles);
        assert_eq!(claims.permissions, cloned.permissions);
        assert_eq!(claims.tenant_id, cloned.tenant_id);
    }

    // SecurityContext tests
    #[test]
    fn test_security_context_creation() {
        let context = SecurityContext::new(
            "test-user".to_string(),
            vec![Role::Worker],
            vec![Permission::ReadJobs],
            Some("test-tenant".to_string()),
        );

        assert_eq!(context.subject, "test-user");
        assert_eq!(context.roles.len(), 1);
        assert_eq!(context.permissions.len(), 1);
        assert_eq!(context.tenant_id, Some("test-tenant".to_string()));
    }

    #[test]
    fn test_security_context_has_role() {
        let context = create_admin_context();

        assert!(context.has_role(&Role::Admin));
        assert!(!context.has_role(&Role::Worker));
        assert!(!context.has_role(&Role::Viewer));
    }

    #[test]
    fn test_security_context_has_permission() {
        let context = create_admin_context();

        assert!(context.has_permission(&Permission::ReadJobs));
        assert!(context.has_permission(&Permission::WriteJobs));
        assert!(context.has_permission(&Permission::AdminSystem));
        // Admin context includes all permissions including DeleteJobs
        assert!(context.has_permission(&Permission::DeleteJobs));
    }

    #[test]
    fn test_security_context_is_admin() {
        let admin_context = create_admin_context();
        let worker_context = create_worker_context();
        let viewer_context = create_viewer_context();

        assert!(admin_context.is_admin());

        // Worker with Admin role is also admin
        let worker_as_admin = SecurityContext {
            subject: "worker-user".to_string(),
            roles: vec![Role::Worker, Role::Admin],
            permissions: vec![Permission::ReadJobs],
            tenant_id: Some("test".to_string()),
        };
        assert!(worker_as_admin.is_admin());

        assert!(!worker_context.is_admin());
        assert!(!viewer_context.is_admin());
    }

    #[test]
    fn test_security_context_with_multiple_roles() {
        let context = SecurityContext::new(
            "multi-role-user".to_string(),
            vec![Role::Admin, Role::Worker, Role::Operator],
            vec![Permission::ReadJobs, Permission::WriteJobs],
            None,
        );

        assert!(context.has_role(&Role::Admin));
        assert!(context.has_role(&Role::Worker));
        assert!(context.has_role(&Role::Operator));
        assert!(context.has_permission(&Permission::ReadJobs));
        assert!(context.has_permission(&Permission::WriteJobs));
        assert!(context.is_admin());
    }

    #[test]
    fn test_security_context_with_no_roles() {
        let context = create_anonymous_context();

        assert!(!context.has_role(&Role::Admin));
        assert!(!context.has_role(&Role::Worker));
        assert!(!context.has_permission(&Permission::ReadJobs));
        assert!(!context.is_admin());
    }

    #[test]
    fn test_security_context_system_role_is_admin() {
        let system_context = SecurityContext {
            subject: "system".to_string(),
            roles: vec![Role::System],
            permissions: vec![Permission::AdminSystem],
            tenant_id: Some("system".to_string()),
        };

        assert!(system_context.is_admin());
        assert!(system_context.has_role(&Role::System));
    }

    #[test]
    fn test_security_context_clone() {
        let context = create_admin_context();
        let cloned = context.clone();

        assert_eq!(context.subject, cloned.subject);
        assert_eq!(context.roles, cloned.roles);
        assert_eq!(context.permissions, cloned.permissions);
        assert_eq!(context.tenant_id, cloned.tenant_id);
    }

    #[test]
    fn test_security_context_serialization() {
        let context = create_admin_context();
        let serialized = serde_json::to_string(&context).unwrap();
        let deserialized: SecurityContext = serde_json::from_str(&serialized).unwrap();

        assert_eq!(context.subject, deserialized.subject);
        assert_eq!(context.roles, deserialized.roles);
        assert_eq!(context.permissions, deserialized.permissions);
        assert_eq!(context.tenant_id, deserialized.tenant_id);
    }

    #[test]
    fn test_context_with_many_permissions() {
        let permissions = vec![
            Permission::ReadJobs,
            Permission::WriteJobs,
            Permission::DeleteJobs,
            Permission::ManageWorkers,
            Permission::ViewMetrics,
            Permission::AdminSystem,
        ];

        let context = SecurityContext {
            subject: "admin".to_string(),
            roles: vec![Role::Admin],
            permissions: permissions.clone(),
            tenant_id: Some("test".to_string()),
        };

        for perm in permissions {
            assert!(context.has_permission(&perm));
        }
    }

    #[test]
    fn test_role_permission_matrix() {
        // Admin has all permissions
        let admin = create_admin_context();
        assert!(admin.has_permission(&Permission::ReadJobs));
        assert!(admin.has_permission(&Permission::WriteJobs));
        assert!(admin.has_permission(&Permission::DeleteJobs));
        assert!(admin.has_permission(&Permission::ManageWorkers));
        assert!(admin.has_permission(&Permission::ViewMetrics));
        assert!(admin.has_permission(&Permission::AdminSystem));

        // Worker has limited permissions
        let worker = create_worker_context();
        assert!(worker.has_permission(&Permission::ReadJobs));
        assert!(worker.has_permission(&Permission::WriteJobs));
        assert!(!worker.has_permission(&Permission::DeleteJobs));
        assert!(!worker.has_permission(&Permission::ManageWorkers));
        assert!(!worker.has_permission(&Permission::ViewMetrics));
        assert!(!worker.has_permission(&Permission::AdminSystem));

        // Viewer has minimal permissions
        let viewer = create_viewer_context();
        assert!(viewer.has_permission(&Permission::ReadJobs));
        assert!(!viewer.has_permission(&Permission::WriteJobs));
        assert!(viewer.has_permission(&Permission::ViewMetrics));
    }

    #[test]
    fn test_security_context_equality() {
        let context1 = SecurityContext {
            subject: "user1".to_string(),
            roles: vec![Role::Worker],
            permissions: vec![Permission::ReadJobs],
            tenant_id: Some("tenant1".to_string()),
        };

        let context2 = SecurityContext {
            subject: "user1".to_string(),
            roles: vec![Role::Worker],
            permissions: vec![Permission::ReadJobs],
            tenant_id: Some("tenant1".to_string()),
        };

        let context3 = SecurityContext {
            subject: "user2".to_string(),
            roles: vec![Role::Worker],
            permissions: vec![Permission::ReadJobs],
            tenant_id: Some("tenant1".to_string()),
        };

        assert_eq!(context1.subject, context2.subject);
        assert_eq!(context1.roles, context2.roles);
        assert_eq!(context1.permissions, context2.permissions);
        assert_eq!(context1.tenant_id, context2.tenant_id);

        assert_ne!(context1.subject, context3.subject);
    }

    #[test]
    fn test_empty_roles_and_permissions() {
        let context = SecurityContext::new("empty-user".to_string(), vec![], vec![], None);

        assert!(context.roles.is_empty());
        assert!(context.permissions.is_empty());
        assert!(!context.is_admin());
        assert!(!context.has_role(&Role::Admin));
        assert!(!context.has_permission(&Permission::ReadJobs));
    }
}
