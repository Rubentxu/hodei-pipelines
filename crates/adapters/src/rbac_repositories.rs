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
        let mut roles = self.roles.write().unwrap();
        roles.insert(role.id.clone(), role.clone());
        Ok(())
    }

    async fn get_role(&self, id: &RoleId) -> Result<Option<RoleEntity>, DomainError> {
        let roles = self.roles.read().unwrap();
        Ok(roles.get(id).cloned())
    }

    async fn get_role_by_name(&self, name: &str) -> Result<Option<RoleEntity>, DomainError> {
        let roles = self.roles.read().unwrap();
        Ok(roles.values().find(|r| r.name == name).cloned())
    }

    async fn list_all_roles(&self) -> Result<Vec<RoleEntity>, DomainError> {
        let roles = self.roles.read().unwrap();
        Ok(roles.values().cloned().collect())
    }

    async fn delete_role(&self, id: &RoleId) -> Result<(), DomainError> {
        let mut roles = self.roles.write().unwrap();
        roles.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &RoleId) -> Result<bool, DomainError> {
        let roles = self.roles.read().unwrap();
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
        let mut permissions = self.permissions.write().unwrap();
        permissions.insert(permission.id.clone(), permission.clone());
        Ok(())
    }

    async fn get_permission(
        &self,
        id: &PermissionId,
    ) -> Result<Option<PermissionEntity>, DomainError> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions.get(id).cloned())
    }

    async fn get_permission_by_key(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<PermissionEntity>, DomainError> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions
            .values()
            .find(|p| p.resource == resource && p.action == action)
            .cloned())
    }

    async fn list_all_permissions(&self) -> Result<Vec<PermissionEntity>, DomainError> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions.values().cloned().collect())
    }

    async fn delete_permission(&self, id: &PermissionId) -> Result<(), DomainError> {
        let mut permissions = self.permissions.write().unwrap();
        permissions.remove(id);
        Ok(())
    }

    async fn exists(&self, id: &PermissionId) -> Result<bool, DomainError> {
        let permissions = self.permissions.read().unwrap();
        Ok(permissions.contains_key(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodei_core::security::{PermissionId, RoleId};

    #[tokio::test]
    async fn test_save_and_get_role() {
        let repo = InMemoryRoleRepository::new();
        let role = RoleEntity::new(
            RoleId::new(),
            "TestRole".to_string(),
            "Test Description".to_string(),
            vec![],
        )
        .unwrap();

        repo.save_role(&role).await.unwrap();
        let retrieved = repo.get_role(&role.id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "TestRole");
    }

    #[tokio::test]
    async fn test_delete_role() {
        let repo = InMemoryRoleRepository::new();
        let role = RoleEntity::new(
            RoleId::new(),
            "TestRole".to_string(),
            "Test Description".to_string(),
            vec![],
        )
        .unwrap();

        repo.save_role(&role).await.unwrap();
        repo.delete_role(&role.id).await.unwrap();

        let retrieved = repo.get_role(&role.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_save_and_get_permission() {
        let repo = InMemoryPermissionRepository::new();
        let permission = PermissionEntity::new(
            PermissionId::new(),
            "TestPermission".to_string(),
            "Test Description".to_string(),
            "resource".to_string(),
            "action".to_string(),
        )
        .unwrap();

        repo.save_permission(&permission).await.unwrap();
        let retrieved = repo.get_permission(&permission.id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().resource, "resource");
    }

    #[tokio::test]
    async fn test_get_permission_by_key() {
        let repo = InMemoryPermissionRepository::new();
        let permission = PermissionEntity::new(
            PermissionId::new(),
            "TestPermission".to_string(),
            "Test Description".to_string(),
            "pipeline".to_string(),
            "read".to_string(),
        )
        .unwrap();

        repo.save_permission(&permission).await.unwrap();
        let retrieved = repo
            .get_permission_by_key("pipeline", "read")
            .await
            .unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().key(), "pipeline:read");
    }
}
