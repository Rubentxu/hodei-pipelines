//! PostgreSQL RBAC Repositories
//!
//! Production-ready implementations of RoleRepository and PermissionRepository
//! using PostgreSQL with SQLx for persistence.
//!
//! Applies production-ready optimizations:
//! - Structured logging with tracing
//! - Optimized constants
//! - Transaction consistency
//! - Proper error handling with DomainError

use async_trait::async_trait;
use hodei_pipelines_core::{
    DomainError,
    security::{PermissionEntity, PermissionId, RoleEntity, RoleId},
};
use hodei_pipelines_ports::{PermissionRepository, RoleRepository};
use sqlx::{PgPool, Row};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// PostgreSQL Role Repository
/// Implements production-ready persistence for RBAC roles
#[derive(Debug, Clone)]
pub struct PostgreSqlRoleRepository {
    pool: PgPool,
}

// Constants for optimization and maintainability
mod constants {
    pub const LOG_PREFIX: &str = "[PostgreSqlRoleRepository]";
    pub const ROLE_NOT_FOUND: &str = "Role not found";
    pub const ROLE_EXISTS_CHECK: &str = "SELECT EXISTS(SELECT 1 FROM rbac_roles WHERE id = $1)";
    pub const ROLE_FIND_BY_ID: &str =
        "SELECT id, name, description, created_at, updated_at FROM rbac_roles WHERE id = $1";
    pub const ROLE_FIND_BY_NAME: &str =
        "SELECT id, name, description, created_at, updated_at FROM rbac_roles WHERE name = $1";
    pub const ROLE_LIST_ALL: &str =
        "SELECT id, name, description, created_at, updated_at FROM rbac_roles ORDER BY name";
    pub const ROLE_INSERT: &str =
        "INSERT INTO rbac_roles (id, name, description) VALUES ($1, $2, $3)";
    pub const _ROLE_UPDATE: &str = "UPDATE rbac_roles SET name = COALESCE($2, name), description = COALESCE($3, description), updated_at = $4 WHERE id = $1";
    pub const ROLE_DELETE: &str = "DELETE FROM rbac_roles WHERE id = $1";
    pub const ROLE_PERMISSIONS_FIND: &str =
        "SELECT permission_id FROM rbac_role_permissions WHERE role_id = $1";
    pub const ROLE_PERMISSIONS_INSERT: &str = "INSERT INTO rbac_role_permissions (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING";
    pub const ROLE_PERMISSIONS_DELETE: &str =
        "DELETE FROM rbac_role_permissions WHERE role_id = $1";
}

impl PostgreSqlRoleRepository {
    /// Create a new PostgreSQL role repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Save a role with its permissions in a transaction
    async fn save_role_with_permissions(&self, role: &RoleEntity) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            error!(
                "{} Failed to begin transaction for saving role {}: {}",
                constants::LOG_PREFIX,
                role.id,
                e
            );
            DomainError::Infrastructure(format!(
                "Failed to begin transaction for role {}: {}",
                role.id, e
            ))
        })?;

        // Insert or update role
        sqlx::query(constants::ROLE_INSERT)
            .bind(role.id.as_uuid())
            .bind(&role.name)
            .bind(&role.description)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to insert/update role {}: {}",
                    constants::LOG_PREFIX,
                    role.id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to save role {}: {}", role.id, e))
            })?;

        // Delete existing permissions
        sqlx::query(constants::ROLE_PERMISSIONS_DELETE)
            .bind(role.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to delete existing permissions for role {}: {}",
                    constants::LOG_PREFIX,
                    role.id,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to update permissions for role {}: {}",
                    role.id, e
                ))
            })?;

        // Insert new permissions
        for permission_id in &role.permissions {
            sqlx::query(constants::ROLE_PERMISSIONS_INSERT)
                .bind(role.id.as_uuid())
                .bind(permission_id.as_uuid())
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    error!(
                        "{} Failed to insert permission {:?} for role {}: {}",
                        constants::LOG_PREFIX,
                        permission_id,
                        role.id,
                        e
                    );
                    DomainError::Infrastructure(format!(
                        "Failed to assign permission {:?} to role {}: {}",
                        permission_id, role.id, e
                    ))
                })?;
        }

        // Commit transaction
        tx.commit().await.map_err(|e| {
            error!(
                "{} Failed to commit transaction for role {}: {}",
                constants::LOG_PREFIX,
                role.id,
                e
            );
            DomainError::Infrastructure(format!(
                "Failed to commit transaction for role {}: {}",
                role.id, e
            ))
        })?;

        info!(
            "{} Role {} saved successfully with {} permissions",
            constants::LOG_PREFIX,
            role.id,
            role.permissions.len()
        );

        Ok(())
    }

    /// Load a role with its permissions
    async fn load_role_with_permissions(
        &self,
        id: &RoleId,
    ) -> Result<Option<RoleEntity>, DomainError> {
        // Load role basic info
        let role_row = sqlx::query(constants::ROLE_FIND_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query role {}: {}",
                    constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query role {}: {}", id, e))
            })?;

        let Some(role) = role_row else {
            return Ok(None);
        };

        // Load permissions
        let permission_rows = sqlx::query(constants::ROLE_PERMISSIONS_FIND)
            .bind(id.as_uuid())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query permissions for role {}: {}",
                    constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to query permissions for role {}: {}",
                    id, e
                ))
            })?;

        let permissions = permission_rows
            .into_iter()
            .map(|row| {
                let permission_id: Uuid = row.get("permission_id");
                PermissionId::from_uuid(permission_id)
            })
            .collect();

        let role = RoleEntity {
            id: RoleId::from_uuid(role.get("id")),
            name: role.get("name"),
            description: role.get("description"),
            permissions,
            created_at: role.get("created_at"),
            updated_at: role.get("updated_at"),
        };

        Ok(Some(role))
    }
}

#[async_trait]
impl RoleRepository for PostgreSqlRoleRepository {
    #[instrument(skip(self))]
    async fn save_role(&self, role: &RoleEntity) -> Result<(), DomainError> {
        info!(
            "{} Saving role {} with {} permissions",
            constants::LOG_PREFIX,
            role.id,
            role.permissions.len()
        );

        self.save_role_with_permissions(role).await
    }

    #[instrument(skip(self))]
    async fn get_role(&self, id: &RoleId) -> Result<Option<RoleEntity>, DomainError> {
        info!("{} Loading role {}", constants::LOG_PREFIX, id);

        self.load_role_with_permissions(id).await
    }

    #[instrument(skip(self))]
    async fn get_role_by_name(&self, name: &str) -> Result<Option<RoleEntity>, DomainError> {
        info!("{} Loading role by name: {}", constants::LOG_PREFIX, name);

        let role_row = sqlx::query(constants::ROLE_FIND_BY_NAME)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query role by name {}: {}",
                    constants::LOG_PREFIX,
                    name,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query role {}: {}", name, e))
            })?;

        let Some(role) = role_row else {
            return Ok(None);
        };

        let role_id = RoleId::from_uuid(role.get("id"));
        self.load_role_with_permissions(&role_id).await
    }

    #[instrument(skip(self))]
    async fn list_all_roles(&self) -> Result<Vec<RoleEntity>, DomainError> {
        info!("{} Loading all roles", constants::LOG_PREFIX);

        let role_rows = sqlx::query(constants::ROLE_LIST_ALL)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!("{} Failed to list all roles: {}", constants::LOG_PREFIX, e);
                DomainError::Infrastructure(format!("Failed to list roles: {}", e))
            })?;

        let mut roles = Vec::with_capacity(role_rows.len());

        for role_row in role_rows {
            let role_id = RoleId::from_uuid(role_row.get("id"));
            if let Some(role) = self.load_role_with_permissions(&role_id).await? {
                roles.push(role);
            }
        }

        info!("{} Loaded {} roles", constants::LOG_PREFIX, roles.len());

        Ok(roles)
    }

    #[instrument(skip(self))]
    async fn delete_role(&self, id: &RoleId) -> Result<(), DomainError> {
        info!("{} Deleting role {}", constants::LOG_PREFIX, id);

        // Check if role exists
        let exists = sqlx::query(constants::ROLE_EXISTS_CHECK)
            .bind(id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to check if role {} exists: {}",
                    constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to check role existence {}: {}", id, e))
            })?;

        let role_exists: bool = exists.get("exists");

        if !role_exists {
            warn!(
                "{} Role {} not found for deletion",
                constants::LOG_PREFIX,
                id
            );
            return Err(DomainError::NotFound(format!(
                "{} {}",
                constants::ROLE_NOT_FOUND,
                id
            )));
        }

        // Delete role (permissions will be deleted via foreign key cascade)
        sqlx::query(constants::ROLE_DELETE)
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to delete role {}: {}",
                    constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to delete role {}: {}", id, e))
            })?;

        info!("{} Role {} deleted successfully", constants::LOG_PREFIX, id);

        Ok(())
    }

    #[instrument(skip(self))]
    async fn exists(&self, id: &RoleId) -> Result<bool, DomainError> {
        let exists = sqlx::query(constants::ROLE_EXISTS_CHECK)
            .bind(id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to check if role {} exists: {}",
                    constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to check role existence {}: {}", id, e))
            })?;

        let exists: bool = exists.get("exists");
        Ok(exists)
    }
}

/// PostgreSQL Permission Repository
/// Implements production-ready persistence for RBAC permissions
#[derive(Debug, Clone)]
pub struct PostgreSqlPermissionRepository {
    pool: PgPool,
}

// Constants for optimization and maintainability
mod permission_constants {
    pub const LOG_PREFIX: &str = "[PostgreSqlPermissionRepository]";
    pub const PERMISSION_NOT_FOUND: &str = "Permission not found";
    pub const PERMISSION_EXISTS_CHECK: &str =
        "SELECT EXISTS(SELECT 1 FROM rbac_permissions WHERE id = $1)";
    pub const PERMISSION_FIND_BY_ID: &str = "SELECT id, name, description, resource, action, created_at, updated_at FROM rbac_permissions WHERE id = $1";
    pub const PERMISSION_FIND_BY_KEY: &str = "SELECT id, name, description, resource, action, created_at, updated_at FROM rbac_permissions WHERE resource = $1 AND action = $2";
    pub const PERMISSION_LIST_ALL: &str = "SELECT id, name, description, resource, action, created_at, updated_at FROM rbac_permissions ORDER BY resource, action";
    pub const PERMISSION_INSERT: &str = "INSERT INTO rbac_permissions (id, name, description, resource, action) VALUES ($1, $2, $3, $4, $5)";
    pub const _PERMISSION_UPDATE: &str = "UPDATE rbac_permissions SET name = COALESCE($2, name), description = COALESCE($3, description), updated_at = $4 WHERE id = $1";
    pub const PERMISSION_DELETE: &str = "DELETE FROM rbac_permissions WHERE id = $1";
}

impl PostgreSqlPermissionRepository {
    /// Create a new PostgreSQL permission repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PermissionRepository for PostgreSqlPermissionRepository {
    #[instrument(skip(self))]
    async fn save_permission(&self, permission: &PermissionEntity) -> Result<(), DomainError> {
        info!(
            "{} Saving permission {} ({}:{})",
            permission_constants::LOG_PREFIX,
            permission.id,
            permission.resource,
            permission.action
        );

        sqlx::query(permission_constants::PERMISSION_INSERT)
            .bind(permission.id.as_uuid())
            .bind(&permission.name)
            .bind(&permission.description)
            .bind(&permission.resource)
            .bind(&permission.action)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to save permission {}: {}",
                    permission_constants::LOG_PREFIX,
                    permission.id,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to save permission {}: {}",
                    permission.id, e
                ))
            })?;

        info!(
            "{} Permission {} saved successfully",
            permission_constants::LOG_PREFIX,
            permission.id
        );

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_permission(
        &self,
        id: &PermissionId,
    ) -> Result<Option<PermissionEntity>, DomainError> {
        info!(
            "{} Loading permission {}",
            permission_constants::LOG_PREFIX,
            id
        );

        let permission_row = sqlx::query(permission_constants::PERMISSION_FIND_BY_ID)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query permission {}: {}",
                    permission_constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to query permission {}: {}", id, e))
            })?;

        let Some(permission) = permission_row else {
            return Ok(None);
        };

        let permission = PermissionEntity {
            id: PermissionId::from_uuid(permission.get("id")),
            name: permission.get("name"),
            description: permission.get("description"),
            resource: permission.get("resource"),
            action: permission.get("action"),
            created_at: permission.get("created_at"),
            updated_at: permission.get("updated_at"),
        };

        Ok(Some(permission))
    }

    #[instrument(skip(self))]
    async fn get_permission_by_key(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<PermissionEntity>, DomainError> {
        info!(
            "{} Loading permission by key {}:{}",
            permission_constants::LOG_PREFIX,
            resource,
            action
        );

        let permission_row = sqlx::query(permission_constants::PERMISSION_FIND_BY_KEY)
            .bind(resource)
            .bind(action)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to query permission by key {}:{}: {}",
                    permission_constants::LOG_PREFIX,
                    resource,
                    action,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to query permission {}:{}: {}",
                    resource, action, e
                ))
            })?;

        let Some(permission) = permission_row else {
            return Ok(None);
        };

        let permission = PermissionEntity {
            id: PermissionId::from_uuid(permission.get("id")),
            name: permission.get("name"),
            description: permission.get("description"),
            resource: permission.get("resource"),
            action: permission.get("action"),
            created_at: permission.get("created_at"),
            updated_at: permission.get("updated_at"),
        };

        Ok(Some(permission))
    }

    #[instrument(skip(self))]
    async fn list_all_permissions(&self) -> Result<Vec<PermissionEntity>, DomainError> {
        info!(
            "{} Loading all permissions",
            permission_constants::LOG_PREFIX
        );

        let permission_rows = sqlx::query(permission_constants::PERMISSION_LIST_ALL)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to list all permissions: {}",
                    permission_constants::LOG_PREFIX,
                    e
                );
                DomainError::Infrastructure(format!("Failed to list permissions: {}", e))
            })?;

        let mut permissions = Vec::with_capacity(permission_rows.len());

        for permission_row in permission_rows {
            let permission = PermissionEntity {
                id: PermissionId::from_uuid(permission_row.get("id")),
                name: permission_row.get("name"),
                description: permission_row.get("description"),
                resource: permission_row.get("resource"),
                action: permission_row.get("action"),
                created_at: permission_row.get("created_at"),
                updated_at: permission_row.get("updated_at"),
            };
            permissions.push(permission);
        }

        info!(
            "{} Loaded {} permissions",
            permission_constants::LOG_PREFIX,
            permissions.len()
        );

        Ok(permissions)
    }

    #[instrument(skip(self))]
    async fn delete_permission(&self, id: &PermissionId) -> Result<(), DomainError> {
        info!(
            "{} Deleting permission {}",
            permission_constants::LOG_PREFIX,
            id
        );

        // Check if permission exists
        let exists = sqlx::query(permission_constants::PERMISSION_EXISTS_CHECK)
            .bind(id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to check if permission {} exists: {}",
                    permission_constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to check permission existence {}: {}",
                    id, e
                ))
            })?;

        let permission_exists: bool = exists.get("exists");

        if !permission_exists {
            warn!(
                "{} Permission {} not found for deletion",
                permission_constants::LOG_PREFIX,
                id
            );
            return Err(DomainError::NotFound(format!(
                "{} {}",
                permission_constants::PERMISSION_NOT_FOUND,
                id
            )));
        }

        // Delete permission (role-permission mappings will be deleted via foreign key cascade)
        sqlx::query(permission_constants::PERMISSION_DELETE)
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to delete permission {}: {}",
                    permission_constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!("Failed to delete permission {}: {}", id, e))
            })?;

        info!(
            "{} Permission {} deleted successfully",
            permission_constants::LOG_PREFIX,
            id
        );

        Ok(())
    }

    #[instrument(skip(self))]
    async fn exists(&self, id: &PermissionId) -> Result<bool, DomainError> {
        let exists = sqlx::query(permission_constants::PERMISSION_EXISTS_CHECK)
            .bind(id.as_uuid())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                error!(
                    "{} Failed to check if permission {} exists: {}",
                    permission_constants::LOG_PREFIX,
                    id,
                    e
                );
                DomainError::Infrastructure(format!(
                    "Failed to check permission existence {}: {}",
                    id, e
                ))
            })?;

        let exists: bool = exists.get("exists");
        Ok(exists)
    }
}
