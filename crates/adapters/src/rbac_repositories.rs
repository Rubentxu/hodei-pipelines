//! In-memory repositories for RBAC
//!
//! Provides in-memory implementations of RoleRepository and PermissionRepository
//! for testing and development purposes.

use async_trait::async_trait;
use hodei_core::{
    DomainError,
    security::{PermissionEntity, PermissionId, RoleEntity, RoleId},
};
use hodei_ports::{PermissionRepository, RoleRepository};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory Role Repository
#[derive(Debug, Clone)]
pub struct InMemoryRoleRepository {
    roles: Arc<RwLock<HashMap<RoleId, RoleEntity>>>,
}

impl InMemoryRoleRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryRoleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoleRepository for InMemoryRoleRepository {
    async fn save_role(&self, role: &RoleEntity) -> Result<(), DomainError> {
        let mut roles = self
            .roles
            .write()
            .expect("Failed to acquire write lock on roles (poisoned lock)");
        roles.insert(role.id.clone(), role.clone());
        Ok(())
    }

    async fn get_role(&self, id: &RoleId) -> Result<Option<RoleEntity>, DomainError> {
        let roles = self
            .roles
            .read()
            .expect("Failed to acquire read lock on roles (poisoned lock)");
        Ok(roles.get(id).cloned())
    }

    async fn get_role_by_name(&self, name: &str) -> Result<Option<RoleEntity>, DomainError> {
        let roles = self
            .roles
            .read()
            .expect("Failed to acquire read lock on roles (poisoned lock)");
        Ok(roles.values().find(|r| r.name == name).cloned())
    }

    async fn list_all_roles(&self) -> Result<Vec<RoleEntity>, DomainError> {
        let roles = self
            .roles
            .read()
            .expect("Failed to acquire read lock on roles (poisoned lock)");
        Ok(roles.values().cloned().collect())
    }

    async fn delete_role(&self, id: &RoleId) -> Result<(), DomainError> {
        let mut roles = self
            .roles
            .write()
            .expect("Failed to acquire write lock on roles (poisoned lock)");
        roles.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &RoleId) -> Result<bool, DomainError> {
        let roles = self
            .roles
            .read()
            .expect("Failed to acquire read lock on roles (poisoned lock)");
        Ok(roles.contains_key(id))
    }
}

/// In-memory Permission Repository
#[derive(Debug, Clone)]
pub struct InMemoryPermissionRepository {
    permissions: Arc<RwLock<HashMap<PermissionId, PermissionEntity>>>,
}

impl InMemoryPermissionRepository {
    /// Create a new empty repository
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryPermissionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PermissionRepository for InMemoryPermissionRepository {
    async fn save_permission(&self, permission: &PermissionEntity) -> Result<(), DomainError> {
        let mut permissions = self
            .permissions
            .write()
            .expect("Failed to acquire write lock on permissions (poisoned lock)");
        permissions.insert(permission.id.clone(), permission.clone());
        Ok(())
    }

    async fn get_permission(
        &self,
        id: &PermissionId,
    ) -> Result<Option<PermissionEntity>, DomainError> {
        let permissions = self
            .permissions
            .read()
            .expect("Failed to acquire read lock on permissions (poisoned lock)");
        Ok(permissions.get(id).cloned())
    }

    async fn get_permission_by_key(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<PermissionEntity>, DomainError> {
        let permissions = self
            .permissions
            .read()
            .expect("Failed to acquire read lock on permissions (poisoned lock)");
        Ok(permissions
            .values()
            .find(|p| p.resource == resource && p.action == action)
            .cloned())
    }

    async fn list_all_permissions(&self) -> Result<Vec<PermissionEntity>, DomainError> {
        let permissions = self
            .permissions
            .read()
            .expect("Failed to acquire read lock on permissions (poisoned lock)");
        Ok(permissions.values().cloned().collect())
    }

    async fn delete_permission(&self, id: &PermissionId) -> Result<(), DomainError> {
        let mut permissions = self
            .permissions
            .write()
            .expect("Failed to acquire write lock on permissions (poisoned lock)");
        permissions.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &PermissionId) -> Result<bool, DomainError> {
        let permissions = self
            .permissions
            .read()
            .expect("Failed to acquire read lock on permissions (poisoned lock)");
        Ok(permissions.contains_key(id))
    }
}

