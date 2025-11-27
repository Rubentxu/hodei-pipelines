//! Role-Based Access Control (RBAC) Module
//!
//! Provides CRUD operations for Role and Permission entities following DDD principles.
//! Implements use cases for managing roles, permissions, and role-permission assignments.

use hodei_core::{
    DomainError,
    security::{
        Permission as LegacyPermission, PermissionEntity, PermissionId, Role as LegacyRole,
        RoleEntity, RoleId,
    },
};
use hodei_ports::{EventPublisher, PermissionRepository, RoleRepository, SystemEvent};
use serde::{Deserialize, Serialize};
use std::result::Result as StdResult;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Configuration for RBAC operations
#[derive(Debug, Clone)]
pub struct RbacConfig {
    pub max_roles_per_user: usize,
    pub max_permissions_per_role: usize,
    pub allow_role_duplication: bool,
}

impl Default for RbacConfig {
    fn default() -> Self {
        Self {
            max_roles_per_user: 10,
            max_permissions_per_role: 100,
            allow_role_duplication: false,
        }
    }
}


// ========== Service ==========

/// RBAC Service - Application layer use cases
pub struct RoleBasedAccessControlService<R, P, E>
where
    R: RoleRepository,
    P: PermissionRepository,
    E: EventPublisher,
{
    role_repo: Arc<R>,
    permission_repo: Arc<P>,
    event_bus: Arc<E>,
    config: RbacConfig,
}

impl<R, P, E> RoleBasedAccessControlService<R, P, E>
where
    R: RoleRepository,
    P: PermissionRepository,
    E: EventPublisher,
{
    /// Create a new RBAC service instance
    pub fn new(
        role_repo: Arc<R>,
        permission_repo: Arc<P>,
        event_bus: Arc<E>,
        config: RbacConfig,
    ) -> Self {
        Self {
            role_repo,
            permission_repo,
            event_bus,
            config,
        }
    }

    /// Create a new role
    pub async fn create_role(&self, request: CreateRoleRequest) -> Result<RoleEntity, RbacError> {
        info!("Creating role: {}", request.name);

        // Validate name length
        if request.name.trim().is_empty() {
            return Err(RbacError::Validation(
                "Role name cannot be empty".to_string(),
            ));
        }
        if request.name.len() > 255 {
            return Err(RbacError::Validation("Role name too long".to_string()));
        }

        // Check if role already exists
        if let Some(existing) = self
            .role_repo
            .get_role_by_name(&request.name)
            .await
            .map_err(RbacError::Repository)?
        {
            return Err(RbacError::RoleAlreadyExists(existing.id));
        }

        // Validate permission count
        if request.permissions.len() > self.config.max_permissions_per_role {
            return Err(RbacError::Validation(format!(
                "Role cannot have more than {} permissions",
                self.config.max_permissions_per_role
            )));
        }

        // Validate permissions exist
        for perm_id in &request.permissions {
            let exists = self
                .permission_repo
                .exists(perm_id)
                .await
                .map_err(RbacError::Repository)?;
            if !exists {
                return Err(RbacError::PermissionNotFound(perm_id.clone()));
            }
        }

        let role = RoleEntity::new(
            RoleId::new(),
            request.name,
            request.description.unwrap_or_default(),
            request.permissions,
        )
        .map_err(RbacError::Domain)?;

        self.role_repo
            .save_role(&role)
            .await
            .map_err(RbacError::Repository)?;

        info!("Role created successfully: {}", role.id);

        Ok(role)
    }

    /// Get role by ID
    pub async fn get_role(&self, id: &RoleId) -> Result<Option<RoleEntity>, RbacError> {
        let role = self
            .role_repo
            .get_role(id)
            .await
            .map_err(RbacError::Repository)?;

        Ok(role)
    }

    /// List all roles
    pub async fn list_roles(&self) -> Result<Vec<RoleEntity>, RbacError> {
        let roles = self
            .role_repo
            .list_all_roles()
            .await
            .map_err(RbacError::Repository)?;

        Ok(roles)
    }

    /// Update role
    pub async fn update_role(
        &self,
        id: &RoleId,
        request: UpdateRoleRequest,
    ) -> Result<RoleEntity, RbacError> {
        info!("Updating role: {}", id);

        let mut role = self
            .role_repo
            .get_role(id)
            .await
            .map_err(RbacError::Repository)?
            .ok_or(RbacError::RoleNotFound(id.clone()))?;

        // Update name if provided
        if let Some(ref new_name) = request.name {
            if new_name.trim().is_empty() {
                return Err(RbacError::Validation(
                    "Role name cannot be empty".to_string(),
                ));
            }
            if new_name.len() > 255 {
                return Err(RbacError::Validation("Role name too long".to_string()));
            }

            // Check if name is already taken by another role
            if let Some(existing) = self
                .role_repo
                .get_role_by_name(new_name)
                .await
                .map_err(RbacError::Repository)?
            {
                if existing.id != *id {
                    return Err(RbacError::RoleAlreadyExists(existing.id));
                }
            }
        }

        // Update role
        role.update(request.name, request.description)
            .map_err(RbacError::Domain)?;

        self.role_repo
            .save_role(&role)
            .await
            .map_err(RbacError::Repository)?;

        // Publish event
        info!("Role updated: {}", id);

        Ok(role)
    }

    /// Delete role
    pub async fn delete_role(&self, id: &RoleId) -> Result<(), RbacError> {
        info!("Deleting role: {}", id);

        // Check if role exists
        let role = self
            .role_repo
            .get_role(id)
            .await
            .map_err(RbacError::Repository)?
            .ok_or(RbacError::RoleNotFound(id.clone()))?;

        // Delete role
        self.role_repo
            .delete_role(id)
            .await
            .map_err(RbacError::Repository)?;

        info!("Role deleted successfully: {}", id);

        Ok(())
    }

    /// Add permission to role
    pub async fn add_permission_to_role(
        &self,
        role_id: &RoleId,
        permission_id: &PermissionId,
    ) -> Result<(), RbacError> {
        let mut role = self
            .role_repo
            .get_role(role_id)
            .await
            .map_err(RbacError::Repository)?
            .ok_or(RbacError::RoleNotFound(role_id.clone()))?;

        // Check if permission exists
        let permission_exists = self
            .permission_repo
            .exists(permission_id)
            .await
            .map_err(RbacError::Repository)?;
        if !permission_exists {
            return Err(RbacError::PermissionNotFound(permission_id.clone()));
        }

        // Add permission
        role.add_permission(permission_id.clone());

        self.role_repo
            .save_role(&role)
            .await
            .map_err(RbacError::Repository)?;

        info!("Added permission to role: {} -> {}", role_id, permission_id);

        Ok(())
    }

    /// Remove permission from role
    pub async fn remove_permission_from_role(
        &self,
        role_id: &RoleId,
        permission_id: &PermissionId,
    ) -> Result<(), RbacError> {
        let mut role = self
            .role_repo
            .get_role(role_id)
            .await
            .map_err(RbacError::Repository)?
            .ok_or(RbacError::RoleNotFound(role_id.clone()))?;

        // Remove permission
        role.remove_permission(permission_id);

        self.role_repo
            .save_role(&role)
            .await
            .map_err(RbacError::Repository)?;

        info!(
            "Removed permission from role: {} -> {}",
            role_id, permission_id
        );

        Ok(())
    }

    /// Create a new permission
    pub async fn create_permission(
        &self,
        request: CreatePermissionRequest,
    ) -> Result<PermissionEntity, RbacError> {
        info!(
            "Creating permission: {}:{}",
            request.resource, request.action
        );

        // Validate
        if request.name.trim().is_empty() {
            return Err(RbacError::Validation(
                "Permission name cannot be empty".to_string(),
            ));
        }
        if request.resource.trim().is_empty() {
            return Err(RbacError::Validation(
                "Permission resource cannot be empty".to_string(),
            ));
        }
        if request.action.trim().is_empty() {
            return Err(RbacError::Validation(
                "Permission action cannot be empty".to_string(),
            ));
        }

        // Check if permission already exists
        if let Some(existing) = self
            .permission_repo
            .get_permission_by_key(&request.resource, &request.action)
            .await
            .map_err(RbacError::Repository)?
        {
            return Err(RbacError::PermissionAlreadyExists(existing.id));
        }

        let permission = PermissionEntity::new(
            PermissionId::new(),
            request.name,
            request.description.unwrap_or_default(),
            request.resource,
            request.action,
        )
        .map_err(RbacError::Domain)?;

        self.permission_repo
            .save_permission(&permission)
            .await
            .map_err(RbacError::Repository)?;

        info!("Permission created successfully: {}", permission.id);

        Ok(permission)
    }

    /// List all permissions
    pub async fn list_permissions(&self) -> Result<Vec<PermissionEntity>, RbacError> {
        let permissions = self
            .permission_repo
            .list_all_permissions()
            .await
            .map_err(RbacError::Repository)?;

        Ok(permissions)
    }

    /// Get permissions for a role
    pub async fn get_role_permissions(
        &self,
        role_id: &RoleId,
    ) -> Result<Vec<PermissionEntity>, RbacError> {
        let role = self
            .role_repo
            .get_role(role_id)
            .await
            .map_err(RbacError::Repository)?
            .ok_or(RbacError::RoleNotFound(role_id.clone()))?;

        let mut permissions = Vec::new();
        for perm_id in &role.permissions {
            let permission = self
                .permission_repo
                .get_permission(perm_id)
                .await
                .map_err(RbacError::Repository)?
                .ok_or(RbacError::PermissionNotFound(perm_id.clone()))?;
            permissions.push(permission);
        }

        Ok(permissions)
    }
}

impl<R, P, E> Clone for RoleBasedAccessControlService<R, P, E>
where
    R: RoleRepository,
    P: PermissionRepository,
    E: EventPublisher,
{
    fn clone(&self) -> Self {
        Self {
            role_repo: self.role_repo.clone(),
            permission_repo: self.permission_repo.clone(),
            event_bus: self.event_bus.clone(),
            config: self.config.clone(),
        }
    }
}

// ========== DTOs ==========

/// Request to create a new role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<PermissionId>,
}

/// Request to update a role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request to create a new permission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePermissionRequest {
    pub name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
}

/// Response for role data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permission_count: usize,
    pub permissions: Vec<PermissionResponse>,
    pub created_at: String,
    pub updated_at: String,
}

/// Response for permission data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub resource: String,
    pub action: String,
    pub key: String,
    pub created_at: String,
    pub updated_at: String,
}

/// List response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRolesResponse {
    pub roles: Vec<RoleResponse>,
    pub total: usize,
}

// ========== Error Types ==========

#[derive(thiserror::Error, Debug)]
pub enum RbacError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Domain error: {0}")]
    Domain(DomainError),

    #[error("Repository error: {0}")]
    Repository(DomainError),

    #[error("Event bus error: {0}")]
    EventBus(hodei_ports::EventBusError),

    #[error("Role not found: {0}")]
    RoleNotFound(RoleId),

    #[error("Permission not found: {0}")]
    PermissionNotFound(PermissionId),

    #[error("Role already exists: {0:?}")]
    RoleAlreadyExists(RoleId),

    #[error("Permission already exists: {0:?}")]
    PermissionAlreadyExists(PermissionId),
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_adapters::InMemoryBus;

    // Mock repositories
    #[derive(Debug, Clone)]
    struct MockRoleRepository {
        roles: Arc<tokio::sync::RwLock<Vec<RoleEntity>>>,
    }

    impl MockRoleRepository {
        fn new() -> Self {
            Self {
                roles: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl RoleRepository for MockRoleRepository {
        async fn save_role(&self, role: &RoleEntity) -> StdResult<(), DomainError> {
            let mut roles = self.roles.write().await;
            if let Some(pos) = roles.iter().position(|r| r.id == role.id) {
                roles[pos] = role.clone();
            } else {
                roles.push(role.clone());
            }
            Ok(())
        }

        async fn get_role(&self, id: &RoleId) -> StdResult<Option<RoleEntity>, DomainError> {
            let roles = self.roles.read().await;
            Ok(roles.iter().find(|r| r.id == *id).cloned())
        }

        async fn get_role_by_name(&self, name: &str) -> StdResult<Option<RoleEntity>, DomainError> {
            let roles = self.roles.read().await;
            Ok(roles.iter().find(|r| r.name == name).cloned())
        }

        async fn list_all_roles(&self) -> StdResult<Vec<RoleEntity>, DomainError> {
            let roles = self.roles.read().await;
            Ok(roles.clone())
        }

        async fn delete_role(&self, id: &RoleId) -> StdResult<(), DomainError> {
            let mut roles = self.roles.write().await;
            roles.retain(|r| r.id != *id);
            Ok(())
        }

        async fn exists(&self, id: &RoleId) -> StdResult<bool, DomainError> {
            let roles = self.roles.read().await;
            Ok(roles.iter().any(|r| r.id == *id))
        }
    }

    #[derive(Debug, Clone)]
    struct MockPermissionRepository {
        permissions: Arc<tokio::sync::RwLock<Vec<PermissionEntity>>>,
    }

    impl MockPermissionRepository {
        fn new() -> Self {
            Self {
                permissions: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl PermissionRepository for MockPermissionRepository {
        async fn save_permission(
            &self,
            permission: &PermissionEntity,
        ) -> StdResult<(), DomainError> {
            let mut permissions = self.permissions.write().await;
            if let Some(pos) = permissions.iter().position(|p| p.id == permission.id) {
                permissions[pos] = permission.clone();
            } else {
                permissions.push(permission.clone());
            }
            Ok(())
        }

        async fn get_permission(
            &self,
            id: &PermissionId,
        ) -> StdResult<Option<PermissionEntity>, DomainError> {
            let permissions = self.permissions.read().await;
            Ok(permissions.iter().find(|p| p.id == *id).cloned())
        }

        async fn get_permission_by_key(
            &self,
            resource: &str,
            action: &str,
        ) -> StdResult<Option<PermissionEntity>, DomainError> {
            let permissions = self.permissions.read().await;
            Ok(permissions
                .iter()
                .find(|p| p.resource == resource && p.action == action)
                .cloned())
        }

        async fn list_all_permissions(&self) -> StdResult<Vec<PermissionEntity>, DomainError> {
            let permissions = self.permissions.read().await;
            Ok(permissions.clone())
        }

        async fn delete_permission(&self, id: &PermissionId) -> StdResult<(), DomainError> {
            let mut permissions = self.permissions.write().await;
            permissions.retain(|p| p.id != *id);
            Ok(())
        }

        async fn exists(&self, id: &PermissionId) -> StdResult<bool, DomainError> {
            let permissions = self.permissions.read().await;
            Ok(permissions.iter().any(|p| p.id == *id))
        }
    }

    #[tokio::test]
    async fn test_create_role() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        let request = CreateRoleRequest {
            name: "Admin".to_string(),
            description: Some("Administrator role".to_string()),
            permissions: vec![],
        };

        let role = service.create_role(request).await.unwrap();

        assert_eq!(role.name, "Admin");
        assert_eq!(role.description, "Administrator role");
        assert!(role.permissions.is_empty());
    }

    #[tokio::test]
    async fn test_get_role() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        // Create a role
        let request = CreateRoleRequest {
            name: "Viewer".to_string(),
            description: Some("Read-only access".to_string()),
            permissions: vec![],
        };
        let created = service.create_role(request).await.unwrap();

        // Get the role
        let retrieved = service.get_role(&created.id).await.unwrap();

        assert!(retrieved.is_some());
        let role = retrieved.unwrap();
        assert_eq!(role.id, created.id);
        assert_eq!(role.name, "Viewer");
    }

    #[tokio::test]
    async fn test_create_duplicate_role_fails() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        let request = CreateRoleRequest {
            name: "Duplicate".to_string(),
            description: None,
            permissions: vec![],
        };

        // Create role twice
        service.create_role(request.clone()).await.unwrap();
        let result = service.create_role(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_role() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        // Create a role
        let request = CreateRoleRequest {
            name: "Original".to_string(),
            description: None,
            permissions: vec![],
        };
        let created = service.create_role(request).await.unwrap();

        // Update the role
        let update_request = UpdateRoleRequest {
            name: Some("Updated".to_string()),
            description: Some("Updated description".to_string()),
        };
        let updated = service
            .update_role(&created.id, update_request)
            .await
            .unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, "Updated description");
    }

    #[tokio::test]
    async fn test_delete_role() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        // Create a role
        let request = CreateRoleRequest {
            name: "ToDelete".to_string(),
            description: None,
            permissions: vec![],
        };
        let created = service.create_role(request).await.unwrap();

        // Delete the role
        service.delete_role(&created.id).await.unwrap();

        // Verify it's deleted
        let result = service.get_role(&created.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_add_permission_to_role() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        // Create a role
        let role_request = CreateRoleRequest {
            name: "TestRole".to_string(),
            description: None,
            permissions: vec![],
        };
        let role = service.create_role(role_request).await.unwrap();

        // Create a permission
        let perm_request = CreatePermissionRequest {
            name: "Read".to_string(),
            description: None,
            resource: "pipeline".to_string(),
            action: "read".to_string(),
        };
        let permission = service.create_permission(perm_request).await.unwrap();

        // Add permission to role
        service
            .add_permission_to_role(&role.id, &permission.id)
            .await
            .unwrap();

        // Verify permission was added
        let role_permissions = service.get_role_permissions(&role.id).await.unwrap();
        assert_eq!(role_permissions.len(), 1);
        assert_eq!(role_permissions[0].id, permission.id);
    }

    #[tokio::test]
    async fn test_list_roles() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        // Create multiple roles
        for i in 0..3 {
            let request = CreateRoleRequest {
                name: format!("Role{}", i),
                description: None,
                permissions: vec![],
            };
            service.create_role(request).await.unwrap();
        }

        // List all roles
        let roles = service.list_roles().await.unwrap();
        assert_eq!(roles.len(), 3);
    }

    #[tokio::test]
    async fn test_create_permission() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        let request = CreatePermissionRequest {
            name: "Write".to_string(),
            description: None,
            resource: "pipeline".to_string(),
            action: "write".to_string(),
        };

        let permission = service.create_permission(request).await.unwrap();

        assert_eq!(permission.resource, "pipeline");
        assert_eq!(permission.action, "write");
        assert_eq!(permission.key(), "pipeline:write");
    }

    #[tokio::test]
    async fn test_list_permissions() {
        let role_repo = Arc::new(MockRoleRepository::new());
        let permission_repo = Arc::new(MockPermissionRepository::new());
        let event_bus = Arc::new(InMemoryBus::new(100));
        let service = RoleBasedAccessControlService::new(
            role_repo.clone(),
            permission_repo.clone(),
            event_bus.clone(),
            RbacConfig::default(),
        );

        // Create multiple permissions
        for i in 0..3 {
            let request = CreatePermissionRequest {
                name: format!("Perm{}", i),
                description: None,
                resource: "resource".to_string(),
                action: format!("action{}", i),
            };
            service.create_permission(request).await.unwrap();
        }

        // List all permissions
        let permissions = service.list_permissions().await.unwrap();
        assert_eq!(permissions.len(), 3);
    }
}
