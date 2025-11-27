//! Security Management Module
//!
//! Provides CRUD operations for User entities following DDD principles.
//! Implements use cases for managing users, roles, and permissions.

use hodei_core::{
    DomainError,
    security::{Email, Permission, Role, User, UserId, UserStatus},
};
use hodei_ports::{EventPublisher, SystemEvent};
use serde::{Deserialize, Serialize};
use std::result::Result as StdResult;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Configuration for Security Management operations
#[derive(Debug, Clone)]
pub struct SecurityManagementConfig {
    pub max_name_length: usize,
    pub max_roles_per_user: usize,
    pub allow_role_duplication: bool,
}

impl Default for SecurityManagementConfig {
    fn default() -> Self {
        Self {
            max_name_length: 255,
            max_roles_per_user: 10,
            allow_role_duplication: false,
        }
    }
}

/// Request DTO for creating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub status: Option<String>,
    pub roles: Option<Vec<String>>,
}

/// Request DTO for updating a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
}

/// Response DTO for user data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_login: Option<String>,
}

/// List response with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<UserResponse>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// Filter for listing users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUsersFilter {
    pub status: Option<String>,
    pub role: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
}

/// Error types for security management operations
#[derive(Debug, thiserror::Error)]
pub enum SecurityManagementError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Domain error: {0}")]
    DomainError(String),

    #[error("Repository error: {0}")]
    RepositoryError(String),

    #[error("User not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
}

impl From<DomainError> for SecurityManagementError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::InvalidInput(msg) => SecurityManagementError::Validation(msg),
            _ => SecurityManagementError::DomainError(error.to_string()),
        }
    }
}

/// User Repository Port
#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: &User) -> StdResult<(), String>;
    async fn find_by_id(&self, id: &UserId) -> StdResult<Option<User>, String>;
    async fn find_by_email(&self, email: &Email) -> StdResult<Option<User>, String>;
    async fn list_all(&self) -> StdResult<Vec<User>, String>;
    async fn delete(&self, id: &UserId) -> StdResult<(), String>;
    async fn exists(&self, id: &UserId) -> StdResult<bool, String>;
}

/// Security Management Service - Application layer use cases
pub struct SecurityManagementService<R, E>
where
    R: UserRepository,
    E: EventPublisher,
{
    user_repo: Arc<R>,
    event_bus: Arc<E>,
    config: SecurityManagementConfig,
}

impl<R, E> SecurityManagementService<R, E>
where
    R: UserRepository,
    E: EventPublisher,
{
    pub fn new(user_repo: Arc<R>, event_bus: Arc<E>, config: SecurityManagementConfig) -> Self {
        Self {
            user_repo,
            event_bus,
            config,
        }
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        request: CreateUserRequest,
    ) -> Result<UserResponse, SecurityManagementError> {
        info!("Creating user with email: {}", request.email);

        // Validate email
        let email = Email::new(request.email.clone())
            .map_err(|_| SecurityManagementError::Validation("Invalid email format".to_string()))?;

        // Check if user already exists
        if let Ok(Some(_)) = self.user_repo.find_by_email(&email).await {
            return Err(SecurityManagementError::Validation(
                "User with this email already exists".to_string(),
            ));
        }

        // Validate name length
        if request.name.len() > self.config.max_name_length {
            return Err(SecurityManagementError::Validation(format!(
                "Name exceeds maximum length of {}",
                self.config.max_name_length
            )));
        }

        // Determine status
        let status = match request.status {
            Some(s) => UserStatus::from_str(&s)
                .map_err(|_| SecurityManagementError::Validation("Invalid status".to_string()))?,
            None => UserStatus::PendingActivation,
        };

        // Create user entity
        let mut user = User::new(email, request.name, status)
            .map_err(|e| SecurityManagementError::DomainError(e.to_string()))?;

        // Add roles if provided
        if let Some(roles) = request.roles {
            for role_str in roles {
                let role = Role::from_str(&role_str).map_err(|_| {
                    SecurityManagementError::Validation(format!("Invalid role: {}", role_str))
                })?;
                user.add_role(role)
                    .map_err(|e| SecurityManagementError::Validation(e.to_string()))?;
            }
        }

        // Save to repository
        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        // Publish event
        let _ = self
            .event_bus
            .publish(SystemEvent::UserCreated(user.clone()))
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Get user by ID
    pub async fn get_user(&self, id: String) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        Ok(self.to_response(&user))
    }

    /// Update user
    pub async fn update_user(
        &self,
        id: String,
        request: UpdateUserRequest,
    ) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let mut user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        // Update name if provided
        if let Some(name) = request.name {
            if name.len() > self.config.max_name_length {
                return Err(SecurityManagementError::Validation(format!(
                    "Name exceeds maximum length of {}",
                    self.config.max_name_length
                )));
            }
            user.update_name(name)
                .map_err(|e| SecurityManagementError::Validation(e.to_string()))?;
        }

        // Update email if provided
        if let Some(email_str) = request.email {
            let email = Email::new(email_str).map_err(|_| {
                SecurityManagementError::Validation("Invalid email format".to_string())
            })?;
            user.update_email(email)
                .map_err(|e| SecurityManagementError::Validation(e.to_string()))?;
        }

        // Update status if provided
        if let Some(status_str) = request.status {
            let status = UserStatus::from_str(&status_str)
                .map_err(|_| SecurityManagementError::Validation("Invalid status".to_string()))?;
            match status {
                UserStatus::Active => user.activate(),
                UserStatus::Inactive => user.deactivate(),
                UserStatus::Suspended => user.suspend(),
                UserStatus::PendingActivation => {
                    return Err(SecurityManagementError::InvalidStateTransition(
                        "Cannot set status to PendingActivation via update".to_string(),
                    ));
                }
            }
            .map_err(|e| SecurityManagementError::Validation(e.to_string()))?;
        }

        // Save updated user
        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        // Publish event
        let _ = self
            .event_bus
            .publish(SystemEvent::UserUpdated(user.clone()))
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Delete user
    pub async fn delete_user(&self, id: String) -> Result<(), SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        // Check if user exists
        let exists = self
            .user_repo
            .exists(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        if !exists {
            return Err(SecurityManagementError::NotFound(format!(
                "User not found: {}",
                id
            )));
        }

        // Delete from repository
        self.user_repo
            .delete(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        // Publish event
        let _ = self
            .event_bus
            .publish(SystemEvent::UserDeleted { user_id })
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(())
    }

    /// List users with filtering and pagination
    pub async fn list_users(
        &self,
        filter: ListUsersFilter,
    ) -> Result<ListUsersResponse, SecurityManagementError> {
        let page = filter.page.unwrap_or(1);
        let per_page = filter.per_page.unwrap_or(10);
        let offset = (page - 1) * per_page;

        let mut users = self
            .user_repo
            .list_all()
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        // Apply filters
        if let Some(status_str) = filter.status {
            let status = UserStatus::from_str(&status_str).ok();
            if let Some(status) = status {
                users.retain(|u| u.status == status);
            }
        }

        if let Some(role_str) = filter.role {
            if let Ok(role) = Role::from_str(&role_str) {
                users.retain(|u| u.roles.contains(&role));
            }
        }

        if let Some(email) = filter.email {
            users.retain(|u| {
                u.email
                    .as_str()
                    .to_lowercase()
                    .contains(&email.to_lowercase())
            });
        }

        if let Some(name) = filter.name {
            users.retain(|u| u.name.to_lowercase().contains(&name.to_lowercase()));
        }

        let total = users.len();
        let total_pages = (total as f64 / per_page as f64).ceil() as usize;

        // Apply pagination
        let users: Vec<_> = users
            .into_iter()
            .skip(offset)
            .take(per_page)
            .map(|u| self.to_response(&u))
            .collect();

        Ok(ListUsersResponse {
            users,
            total,
            page,
            per_page,
            total_pages,
        })
    }

    /// Activate user
    pub async fn activate_user(&self, id: String) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let mut user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        user.activate()
            .map_err(|e| SecurityManagementError::InvalidStateTransition(e.to_string()))?;

        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        let _ = self
            .event_bus
            .publish(SystemEvent::UserActivated(user.clone()))
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Deactivate user
    pub async fn deactivate_user(
        &self,
        id: String,
    ) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let mut user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        user.deactivate()
            .map_err(|e| SecurityManagementError::InvalidStateTransition(e.to_string()))?;

        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        let _ = self
            .event_bus
            .publish(SystemEvent::UserDeactivated(user.clone()))
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Suspend user
    pub async fn suspend_user(&self, id: String) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let mut user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        user.suspend()
            .map_err(|e| SecurityManagementError::InvalidStateTransition(e.to_string()))?;

        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        let _ = self
            .event_bus
            .publish(SystemEvent::UserSuspended(user.clone()))
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Add role to user
    pub async fn add_role_to_user(
        &self,
        id: String,
        role: String,
    ) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let mut user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        let role = Role::from_str(&role)
            .map_err(|_| SecurityManagementError::Validation(format!("Invalid role: {}", role)))?;

        user.add_role(role.clone())
            .map_err(|e| SecurityManagementError::Validation(e.to_string()))?;

        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        let _ = self
            .event_bus
            .publish(SystemEvent::UserRoleAdded {
                user_id: user_id.clone(),
                role,
            })
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Remove role from user
    pub async fn remove_role_from_user(
        &self,
        id: String,
        role: String,
    ) -> Result<UserResponse, SecurityManagementError> {
        let user_id = UserId::from_uuid(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| SecurityManagementError::Validation("Invalid UUID".to_string()))?,
        );

        let mut user = self
            .user_repo
            .find_by_id(&user_id)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?
            .ok_or_else(|| SecurityManagementError::NotFound(format!("User not found: {}", id)))?;

        let role = Role::from_str(&role)
            .map_err(|_| SecurityManagementError::Validation(format!("Invalid role: {}", role)))?;

        user.remove_role(&role)
            .map_err(|e| SecurityManagementError::Validation(e.to_string()))?;

        self.user_repo
            .save(&user)
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e))?;

        let _ = self
            .event_bus
            .publish(SystemEvent::UserRoleRemoved {
                user_id: user_id.clone(),
                role,
            })
            .await
            .map_err(|e| SecurityManagementError::RepositoryError(e.to_string()))?;

        Ok(self.to_response(&user))
    }

    /// Convert User entity to UserResponse DTO
    fn to_response(&self, user: &User) -> UserResponse {
        // Generate permissions based on roles
        let permissions = if user.roles.contains(&Role::Administrator) {
            // Administrator gets all permissions
            vec![
                Permission::PipelineCreate,
                Permission::PipelineRead,
                Permission::PipelineUpdate,
                Permission::PipelineDelete,
                Permission::PipelineExecute,
                Permission::WorkerCreate,
                Permission::WorkerRead,
                Permission::WorkerUpdate,
                Permission::WorkerDelete,
                Permission::WorkerManage,
                Permission::UserCreate,
                Permission::UserRead,
                Permission::UserUpdate,
                Permission::UserDelete,
                Permission::RoleManage,
                Permission::SecurityAudit,
                Permission::SecurityView,
                Permission::CostView,
                Permission::CostManage,
                Permission::ObservabilityView,
                Permission::SystemAdmin,
            ]
        } else if let Some(Role::Developer) = user.roles.first() {
            vec![
                Permission::PipelineCreate,
                Permission::PipelineRead,
                Permission::PipelineUpdate,
                Permission::PipelineDelete,
                Permission::PipelineExecute,
                Permission::WorkerCreate,
                Permission::WorkerRead,
                Permission::WorkerUpdate,
            ]
        } else if let Some(Role::Viewer) = user.roles.first() {
            vec![
                Permission::PipelineRead,
                Permission::WorkerRead,
                Permission::ObservabilityView,
            ]
        } else if let Some(Role::SecurityAuditor) = user.roles.first() {
            vec![
                Permission::SecurityAudit,
                Permission::SecurityView,
                Permission::PipelineRead,
                Permission::WorkerRead,
                Permission::ObservabilityView,
                Permission::UserRead,
            ]
        } else {
            vec![]
        };

        UserResponse {
            id: user.id.as_uuid().to_string(),
            email: user.email.as_str().to_string(),
            name: user.name.clone(),
            status: user.status.as_str().to_string(),
            roles: user.roles.iter().map(|r| r.as_str().to_string()).collect(),
            permissions: permissions.iter().map(|p| p.as_str().to_string()).collect(),
            created_at: Some(user.created_at.to_rfc3339()),
            updated_at: Some(user.updated_at.to_rfc3339()),
            last_login: user.last_login.map(|d| d.to_rfc3339()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    struct MockUserRepository {
        users: Arc<RwLock<HashMap<UserId, User>>>,
    }

    impl MockUserRepository {
        fn new() -> Self {
            Self {
                users: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl UserRepository for MockUserRepository {
        async fn save(&self, user: &User) -> StdResult<(), String> {
            let mut users = self.users.write().await;
            users.insert(user.id.clone(), user.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &UserId) -> StdResult<Option<User>, String> {
            let users = self.users.read().await;
            Ok(users.get(id).cloned())
        }

        async fn find_by_email(&self, email: &Email) -> StdResult<Option<User>, String> {
            let users = self.users.read().await;
            Ok(users.values().find(|u| &u.email == email).cloned())
        }

        async fn list_all(&self) -> StdResult<Vec<User>, String> {
            let users = self.users.read().await;
            Ok(users.values().cloned().collect())
        }

        async fn delete(&self, id: &UserId) -> StdResult<(), String> {
            let mut users = self.users.write().await;
            users.remove(id);
            Ok(())
        }

        async fn exists(&self, id: &UserId) -> StdResult<bool, String> {
            let users = self.users.read().await;
            Ok(users.contains_key(id))
        }
    }

    struct MockEventBus;

    #[async_trait::async_trait]
    impl EventPublisher for MockEventBus {
        async fn publish(
            &self,
            _event: SystemEvent,
        ) -> StdResult<(), hodei_ports::event_bus::EventBusError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let repo = Arc::new(MockUserRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service =
            SecurityManagementService::new(repo, event_bus, SecurityManagementConfig::default());

        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            status: Some("active".to_string()),
            roles: Some(vec!["developer".to_string()]),
        };

        let result = service.create_user(request).await;
        assert!(result.is_ok());

        let user = result.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.name, "Test User");
        assert_eq!(user.status, "active");
        assert_eq!(user.roles.len(), 1);
    }

    #[tokio::test]
    async fn test_create_user_invalid_email() {
        let repo = Arc::new(MockUserRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service =
            SecurityManagementService::new(repo, event_bus, SecurityManagementConfig::default());

        let request = CreateUserRequest {
            email: "invalid-email".to_string(),
            name: "Test User".to_string(),
            status: None,
            roles: None,
        };

        let result = service.create_user(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityManagementError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let repo = Arc::new(MockUserRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service =
            SecurityManagementService::new(repo, event_bus, SecurityManagementConfig::default());

        let result = service
            .get_user("00000000-0000-0000-0000-000000000000".to_string())
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityManagementError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_delete_user_success() {
        let repo = Arc::new(MockUserRepository::new());
        let event_bus = Arc::new(MockEventBus);
        let service =
            SecurityManagementService::new(repo, event_bus, SecurityManagementConfig::default());

        // Create user
        let request = CreateUserRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            status: None,
            roles: None,
        };
        let user = service.create_user(request).await.unwrap();

        // Delete user
        let result = service.delete_user(user.id.clone()).await;
        assert!(result.is_ok());

        // Verify user is deleted
        let result = service.get_user(user.id).await;
        assert!(result.is_err());
    }
}
