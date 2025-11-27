//! RBAC Repository Ports
//!
//! Defines the repository interfaces for RBAC operations.

use async_trait::async_trait;
use hodei_core::{
    DomainError,
    security::{PermissionEntity, PermissionId, RoleEntity, RoleId},
};

/// Role Repository Port
#[async_trait]
pub trait RoleRepository: Send + Sync {
    async fn save_role(&self, role: &RoleEntity) -> Result<(), DomainError>;
    async fn get_role(&self, id: &RoleId) -> Result<Option<RoleEntity>, DomainError>;
    async fn get_role_by_name(&self, name: &str) -> Result<Option<RoleEntity>, DomainError>;
    async fn list_all_roles(&self) -> Result<Vec<RoleEntity>, DomainError>;
    async fn delete_role(&self, id: &RoleId) -> Result<(), DomainError>;
    async fn exists(&self, id: &RoleId) -> Result<bool, DomainError>;
}

/// Permission Repository Port
#[async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn save_permission(&self, permission: &PermissionEntity) -> Result<(), DomainError>;
    async fn get_permission(
        &self,
        id: &PermissionId,
    ) -> Result<Option<PermissionEntity>, DomainError>;
    async fn get_permission_by_key(
        &self,
        resource: &str,
        action: &str,
    ) -> Result<Option<PermissionEntity>, DomainError>;
    async fn list_all_permissions(&self) -> Result<Vec<PermissionEntity>, DomainError>;
    async fn delete_permission(&self, id: &PermissionId) -> Result<(), DomainError>;
    async fn exists(&self, id: &PermissionId) -> Result<bool, DomainError>;
}
