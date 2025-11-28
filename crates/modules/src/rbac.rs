//! Role-Based Access Control (RBAC) Module
//!
//! Provides CRUD operations for Role and Permission entities following DDD principles.
//! Implements use cases for managing roles, permissions, and role-permission assignments.

use hodei_core::{
    DomainError, Result,
    security::{
        PermissionEntity, PermissionId,
        RoleEntity, RoleId,
    },
};
use hodei_ports::{EventPublisher, PermissionRepository, RoleRepository};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

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
    pub async fn create_role(&self, request: CreateRoleRequest) -> Result<RoleEntity> {
        info!("Creating role: {}", request.name);

        // Validate name length
        if request.name.trim().is_empty() {
            return Err(RbacError::Validation("Role name cannot be empty".to_string()).into());
        }
        if request.name.len() > 255 {
            return Err(RbacError::Validation("Role name too long".to_string()).into());
        }

        // Check if role already exists
        if let Some(existing) = self
            .role_repo
            .get_role_by_name(&request.name)
            .await
            .map_err(RbacError::Repository)?
        {
            return Err(RbacError::RoleAlreadyExists(existing.id).into());
        }

        // Validate permission count
        if request.permissions.len() > self.config.max_permissions_per_role {
            return Err(RbacError::Validation(format!(
                "Role cannot have more than {} permissions",
                self.config.max_permissions_per_role
            ))
            .into());
        }

        // Validate permissions exist
        for perm_id in &request.permissions {
            let exists = self
                .permission_repo
                .exists(perm_id)
                .await
                .map_err(RbacError::Repository)?;
            if !exists {
                return Err(RbacError::PermissionNotFound(perm_id.clone()).into());
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
    pub async fn get_role(&self, id: &RoleId) -> Result<Option<RoleEntity>> {
        let role = self
            .role_repo
            .get_role(id)
            .await
            .map_err(RbacError::Repository)?;

        Ok(role)
    }

    /// List all roles
    pub async fn list_roles(&self) -> Result<Vec<RoleEntity>> {
        let roles = self
            .role_repo
            .list_all_roles()
            .await
            .map_err(RbacError::Repository)?;

        Ok(roles)
    }

    /// Update role
    pub async fn update_role(&self, id: &RoleId, request: UpdateRoleRequest) -> Result<RoleEntity> {
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
                return Err(RbacError::Validation("Role name cannot be empty".to_string()).into());
            }
            if new_name.len() > 255 {
                return Err(RbacError::Validation("Role name too long".to_string()).into());
            }

            // Check if name is already taken by another role
            if let Some(existing) = self
                .role_repo
                .get_role_by_name(new_name)
                .await
                .map_err(RbacError::Repository)?
                && existing.id != *id {
                    return Err(RbacError::RoleAlreadyExists(existing.id).into());
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
    pub async fn delete_role(&self, id: &RoleId) -> Result<()> {
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
    ) -> Result<()> {
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
            return Err(RbacError::PermissionNotFound(permission_id.clone()).into());
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
    ) -> Result<()> {
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
    ) -> Result<PermissionEntity> {
        info!(
            "Creating permission: {}:{}",
            request.resource, request.action
        );

        // Validate
        if request.name.trim().is_empty() {
            return Err(
                RbacError::Validation("Permission name cannot be empty".to_string()).into(),
            );
        }
        if request.resource.trim().is_empty() {
            return Err(
                RbacError::Validation("Permission resource cannot be empty".to_string()).into(),
            );
        }
        if request.action.trim().is_empty() {
            return Err(
                RbacError::Validation("Permission action cannot be empty".to_string()).into(),
            );
        }

        // Check if permission already exists
        if let Some(existing) = self
            .permission_repo
            .get_permission_by_key(&request.resource, &request.action)
            .await
            .map_err(RbacError::Repository)?
        {
            return Err(RbacError::PermissionAlreadyExists(existing.id).into());
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
    pub async fn list_permissions(&self) -> Result<Vec<PermissionEntity>> {
        let permissions = self
            .permission_repo
            .list_all_permissions()
            .await
            .map_err(RbacError::Repository)?;

        Ok(permissions)
    }

    /// Get permissions for a role
    pub async fn get_role_permissions(&self, role_id: &RoleId) -> Result<Vec<PermissionEntity>> {
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


// Convert RbacError to DomainError
impl From<RbacError> for hodei_core::DomainError {
    fn from(err: RbacError) -> Self {
        hodei_core::DomainError::Other(err.to_string())
    }
}
