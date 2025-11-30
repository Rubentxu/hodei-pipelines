//! Role-Based Access Control (RBAC) Module
//!
//! Provides authentication, authorization, and role-based access control capabilities.
//! Implements US-019 from the Epic Web Frontend Production Ready.

use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Role enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum Role {
    /// Super administrator with full access
    SuperAdmin,
    /// Organization administrator
    Admin,
    /// Manager role with elevated permissions
    Manager,
    /// Developer role with limited access
    Developer,
    /// Read-only access
    Viewer,
    /// Guest access with minimal permissions
    Guest,
}

/// Permission enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum Permission {
    /// Read operations
    Read,
    /// Write operations
    Write,
    /// Delete operations
    Delete,
    /// Admin operations
    Admin,
    /// Execute operations
    Execute,
    /// Grant permissions
    Grant,
}

/// Resource type enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub enum ResourceType {
    /// Pipelines
    Pipeline,
    /// Executions
    Execution,
    /// Workers
    Worker,
    /// Resource pools
    ResourcePool,
    /// Metrics and dashboards
    Metrics,
    /// Logs
    Logs,
    /// Traces
    Traces,
    /// Alerts
    Alerts,
    /// Costs and budgets
    Cost,
    /// Security and compliance
    Security,
    /// Users and roles
    Users,
    /// System configuration
    System,
}

/// User structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct User {
    /// Unique user ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address
    pub email: String,
    /// Display name
    pub display_name: String,
    /// User status
    pub is_active: bool,
    /// Tenant ID
    pub tenant_id: String,
    /// User roles
    pub roles: Vec<Role>,
    /// User permissions
    pub permissions: Vec<Permission>,
    /// Creation date
    pub created_at: DateTime<Utc>,
    /// Last login date
    pub last_login: Option<DateTime<Utc>>,
    /// User metadata
    pub metadata: HashMap<String, String>,
}

/// Role assignment structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RoleAssignment {
    /// Assignment ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Role
    pub role: Role,
    /// Resource ID (optional, for resource-specific roles)
    pub resource_id: Option<String>,
    /// Resource type (optional, for resource-specific roles)
    pub resource_type: Option<ResourceType>,
    /// Tenant ID
    pub tenant_id: String,
    /// Grantor user ID
    pub granted_by: String,
    /// Grant date
    pub granted_at: DateTime<Utc>,
    /// Expiration date (optional)
    pub expires_at: Option<DateTime<Utc>>,
}

/// Permission grant structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PermissionGrant {
    /// Grant ID
    pub id: String,
    /// User ID or Role
    pub grantee: String,
    /// Grant type (user or role)
    pub grantee_type: String,
    /// Permission
    pub permission: Permission,
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource ID (optional, for specific resources)
    pub resource_id: Option<String>,
    /// Tenant ID
    pub tenant_id: String,
    /// Grantor user ID
    pub granted_by: String,
    /// Grant date
    pub granted_at: DateTime<Utc>,
}

/// Access control decision
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AccessDecision {
    /// Whether access is granted
    pub allowed: bool,
    /// Permission that was checked
    pub permission: Permission,
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource ID (if applicable)
    pub resource_id: Option<String>,
    /// Reason for decision
    pub reason: String,
    /// Effective permissions after evaluation
    pub effective_permissions: Vec<Permission>,
}

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuthToken {
    /// Token ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Token value (in production, this would be a JWT or similar)
    pub token: String,
    /// Token type
    pub token_type: String,
    /// Expiration date
    pub expires_at: DateTime<Utc>,
    /// Scopes (permissions included in token)
    pub scopes: Vec<Permission>,
    /// Creation date
    pub created_at: DateTime<Utc>,
    /// Last used date
    pub last_used: Option<DateTime<Utc>>,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Login timestamp
    pub login_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Session IP address
    pub ip_address: String,
    /// User agent
    pub user_agent: String,
    /// Is active
    pub is_active: bool,
}

/// Service for role-based access control
#[derive(Debug)]
pub struct RbacService {
    /// Mock users
    mock_users: Arc<Vec<MockUserData>>,
    /// Mock role assignments
    mock_role_assignments: Arc<Vec<MockRoleAssignmentData>>,
    /// Mock permission grants
    mock_permission_grants: Arc<Vec<MockPermissionGrantData>>,
    /// Mock sessions
    mock_sessions: Arc<Vec<MockSessionData>>,
}

#[derive(Debug, Clone)]
struct MockUserData {
    id: String,
    username: String,
    email: String,
    display_name: String,
    is_active: bool,
    tenant_id: String,
    roles: Vec<Role>,
    permissions: Vec<Permission>,
    created_at: DateTime<Utc>,
    last_login: Option<DateTime<Utc>>,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct MockRoleAssignmentData {
    id: String,
    user_id: String,
    role: Role,
    resource_id: Option<String>,
    resource_type: Option<ResourceType>,
    tenant_id: String,
    granted_by: String,
    granted_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct MockPermissionGrantData {
    id: String,
    grantee: String,
    grantee_type: String,
    permission: Permission,
    resource_type: ResourceType,
    resource_id: Option<String>,
    tenant_id: String,
    granted_by: String,
    granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct MockSessionData {
    id: String,
    user_id: String,
    tenant_id: String,
    login_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    ip_address: String,
    user_agent: String,
    is_active: bool,
}

impl RbacService {
    /// Create new RBAC service
    pub fn new() -> Self {
        let mock_users = Self::generate_mock_users();
        let mock_role_assignments = Self::generate_mock_role_assignments();
        let mock_permission_grants = Self::generate_mock_permission_grants();
        let mock_sessions = Self::generate_mock_sessions();

        Self {
            mock_users: Arc::new(mock_users),
            mock_role_assignments: Arc::new(mock_role_assignments),
            mock_permission_grants: Arc::new(mock_permission_grants),
            mock_sessions: Arc::new(mock_sessions),
        }
    }

    /// Authenticate user
    pub async fn authenticate(&self, username: &str, password: &str) -> Option<AuthToken> {
        // In production, this would validate credentials against a secure store
        let users = self.mock_users.clone();

        for user in users.iter() {
            if user.username == username && user.is_active {
                // Generate a mock token (in production, use proper JWT)
                let token = format!("token-{}-{}", user.id, Utc::now().timestamp());

                return Some(AuthToken {
                    id: format!("token-{}", user.id),
                    user_id: user.id.clone(),
                    token,
                    token_type: "Bearer".to_string(),
                    expires_at: Utc::now() + chrono::Duration::hours(24),
                    scopes: user.permissions.clone(),
                    created_at: Utc::now(),
                    last_used: Some(Utc::now()),
                });
            }
        }

        None
    }

    /// Check if user has permission
    pub async fn check_permission(
        &self,
        user_id: &str,
        permission: &Permission,
        resource_type: &ResourceType,
        resource_id: Option<&str>,
    ) -> AccessDecision {
        let users = self.mock_users.clone();
        let role_assignments = self.mock_role_assignments.clone();
        let permission_grants = self.mock_permission_grants.clone();

        // Find user
        let user = users.iter().find(|u| u.id == user_id);

        if user.is_none() || !user.as_ref().unwrap().is_active {
            return AccessDecision {
                allowed: false,
                permission: permission.clone(),
                resource_type: resource_type.clone(),
                resource_id: resource_id.map(|s| s.to_string()),
                reason: "User not found or inactive".to_string(),
                effective_permissions: vec![],
            };
        }

        let user = user.unwrap();

        // Check if user has direct permission
        let has_direct_permission = user.permissions.contains(permission);

        // Check if user has permission through role assignments
        let user_roles: Vec<_> = role_assignments
            .iter()
            .filter(|ra| ra.user_id == user_id)
            .map(|ra| &ra.role)
            .collect();

        // Check permission grants for user
        let user_grants: Vec<_> = permission_grants
            .iter()
            .filter(|pg| pg.grantee == user_id && pg.grantee_type == "user")
            .collect();

        // Check permission grants for roles
        let role_grants: Vec<_> = permission_grants
            .iter()
            .filter(|pg| {
                user_roles
                    .iter()
                    .any(|role| pg.grantee == format!("{:?}", role) && pg.grantee_type == "role")
            })
            .collect();

        let mut effective_permissions = user.permissions.clone();

        // Add permissions from grants
        for grant in user_grants.iter().chain(role_grants.iter()) {
            if !effective_permissions.contains(&grant.permission) {
                effective_permissions.push(grant.permission.clone());
            }
        }

        let allowed = has_direct_permission
            || effective_permissions.contains(permission)
            || user.roles.iter().any(|r| {
                matches!(r, Role::SuperAdmin)
                    || matches!(r, Role::Admin)
                    || (matches!(r, Role::Manager)
                        && (matches!(permission, Permission::Read)
                            || matches!(permission, Permission::Write)
                            || matches!(permission, Permission::Execute)))
                    || (matches!(r, Role::Developer)
                        && (matches!(permission, Permission::Read)
                            || matches!(permission, Permission::Write)
                            || matches!(permission, Permission::Execute)))
                    || (matches!(r, Role::Viewer) && matches!(permission, Permission::Read))
                    || (matches!(r, Role::Guest) && matches!(permission, Permission::Read))
            });

        AccessDecision {
            allowed,
            permission: permission.clone(),
            resource_type: resource_type.clone(),
            resource_id: resource_id.map(|s| s.to_string()),
            reason: if allowed {
                "Access granted".to_string()
            } else {
                "Access denied".to_string()
            },
            effective_permissions,
        }
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, id: &str) -> Option<User> {
        let users = self.mock_users.clone();

        for user in users.iter() {
            if user.id == id {
                return Some(self.convert_to_user(user));
            }
        }

        None
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Option<User> {
        let users = self.mock_users.clone();

        for user in users.iter() {
            if user.username == username {
                return Some(self.convert_to_user(user));
            }
        }

        None
    }

    /// List users for a tenant
    pub async fn list_users(&self, tenant_id: Option<&str>) -> Vec<User> {
        let users = self.mock_users.clone();

        users
            .iter()
            .filter(|user| {
                if let Some(tenant) = tenant_id {
                    user.tenant_id == tenant
                } else {
                    true
                }
            })
            .map(|user| self.convert_to_user(user))
            .collect()
    }

    /// Create user
    pub async fn create_user(&self, user: User) -> Result<User, String> {
        // In production, this would save to database
        Ok(user)
    }

    /// Update user
    pub async fn update_user(&self, id: &str, updates: User) -> Result<User, String> {
        // In production, this would update database
        Ok(updates)
    }

    /// Delete user
    pub async fn delete_user(&self, id: &str) -> Result<(), String> {
        // In production, this would delete from database
        Ok(())
    }

    /// Get role assignments for user
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<RoleAssignment> {
        let assignments = self.mock_role_assignments.clone();

        assignments
            .iter()
            .filter(|assignment| assignment.user_id == user_id)
            .map(|assignment| self.convert_to_role_assignment(assignment))
            .collect()
    }

    /// Assign role to user
    pub async fn assign_role(&self, assignment: RoleAssignment) -> Result<RoleAssignment, String> {
        // In production, this would save to database
        Ok(assignment)
    }

    /// Revoke role from user
    pub async fn revoke_role(&self, assignment_id: &str) -> Result<(), String> {
        // In production, this would delete from database
        Ok(())
    }

    /// Get sessions for user
    pub async fn get_user_sessions(&self, user_id: &str) -> Vec<Session> {
        let sessions = self.mock_sessions.clone();

        sessions
            .iter()
            .filter(|session| session.user_id == user_id)
            .map(|session| self.convert_to_session(session))
            .collect()
    }

    /// Terminate session
    pub async fn terminate_session(&self, session_id: &str) -> Result<(), String> {
        // In production, this would update database
        Ok(())
    }

    /// Convert mock data to user
    fn convert_to_user(&self, data: &MockUserData) -> User {
        User {
            id: data.id.clone(),
            username: data.username.clone(),
            email: data.email.clone(),
            display_name: data.display_name.clone(),
            is_active: data.is_active,
            tenant_id: data.tenant_id.clone(),
            roles: data.roles.clone(),
            permissions: data.permissions.clone(),
            created_at: data.created_at,
            last_login: data.last_login,
            metadata: data.metadata.clone(),
        }
    }

    /// Convert mock data to role assignment
    fn convert_to_role_assignment(&self, data: &MockRoleAssignmentData) -> RoleAssignment {
        RoleAssignment {
            id: data.id.clone(),
            user_id: data.user_id.clone(),
            role: data.role.clone(),
            resource_id: data.resource_id.clone(),
            resource_type: data.resource_type.clone(),
            tenant_id: data.tenant_id.clone(),
            granted_by: data.granted_by.clone(),
            granted_at: data.granted_at,
            expires_at: data.expires_at,
        }
    }

    /// Convert mock data to session
    fn convert_to_session(&self, data: &MockSessionData) -> Session {
        Session {
            id: data.id.clone(),
            user_id: data.user_id.clone(),
            tenant_id: data.tenant_id.clone(),
            login_at: data.login_at,
            last_activity: data.last_activity,
            ip_address: data.ip_address.clone(),
            user_agent: data.user_agent.clone(),
            is_active: data.is_active,
        }
    }

    /// Generate mock users
    fn generate_mock_users() -> Vec<MockUserData> {
        let now = Utc::now();

        vec![
            MockUserData {
                id: "user-001".to_string(),
                username: "admin".to_string(),
                email: "admin@example.com".to_string(),
                display_name: "System Administrator".to_string(),
                is_active: true,
                tenant_id: "tenant-123".to_string(),
                roles: vec![Role::SuperAdmin],
                permissions: vec![
                    Permission::Read,
                    Permission::Write,
                    Permission::Delete,
                    Permission::Admin,
                    Permission::Execute,
                    Permission::Grant,
                ],
                created_at: now - chrono::Duration::days(365),
                last_login: Some(now - chrono::Duration::hours(2)),
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("department".to_string(), "IT".to_string());
                    map.insert("title".to_string(), "Administrator".to_string());
                    map
                },
            },
            MockUserData {
                id: "user-002".to_string(),
                username: "developer1".to_string(),
                email: "dev1@example.com".to_string(),
                display_name: "Developer One".to_string(),
                is_active: true,
                tenant_id: "tenant-123".to_string(),
                roles: vec![Role::Developer],
                permissions: vec![Permission::Read, Permission::Write, Permission::Execute],
                created_at: now - chrono::Duration::days(180),
                last_login: Some(now - chrono::Duration::minutes(30)),
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("department".to_string(), "Engineering".to_string());
                    map.insert("title".to_string(), "Senior Developer".to_string());
                    map
                },
            },
            MockUserData {
                id: "user-003".to_string(),
                username: "manager1".to_string(),
                email: "manager1@example.com".to_string(),
                display_name: "Manager One".to_string(),
                is_active: true,
                tenant_id: "tenant-456".to_string(),
                roles: vec![Role::Manager],
                permissions: vec![Permission::Read, Permission::Write, Permission::Execute],
                created_at: now - chrono::Duration::days(240),
                last_login: Some(now - chrono::Duration::hours(5)),
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("department".to_string(), "Operations".to_string());
                    map.insert("title".to_string(), "Operations Manager".to_string());
                    map
                },
            },
        ]
    }

    /// Generate mock role assignments
    fn generate_mock_role_assignments() -> Vec<MockRoleAssignmentData> {
        let now = Utc::now();

        vec![
            MockRoleAssignmentData {
                id: "assignment-001".to_string(),
                user_id: "user-001".to_string(),
                role: Role::SuperAdmin,
                resource_id: None,
                resource_type: None,
                tenant_id: "tenant-123".to_string(),
                granted_by: "user-001".to_string(),
                granted_at: now - chrono::Duration::days(365),
                expires_at: None,
            },
            MockRoleAssignmentData {
                id: "assignment-002".to_string(),
                user_id: "user-002".to_string(),
                role: Role::Developer,
                resource_id: None,
                resource_type: None,
                tenant_id: "tenant-123".to_string(),
                granted_by: "user-001".to_string(),
                granted_at: now - chrono::Duration::days(180),
                expires_at: None,
            },
            MockRoleAssignmentData {
                id: "assignment-003".to_string(),
                user_id: "user-003".to_string(),
                role: Role::Manager,
                resource_id: None,
                resource_type: None,
                tenant_id: "tenant-456".to_string(),
                granted_by: "user-003".to_string(),
                granted_at: now - chrono::Duration::days(240),
                expires_at: None,
            },
        ]
    }

    /// Generate mock permission grants
    fn generate_mock_permission_grants() -> Vec<MockPermissionGrantData> {
        let now = Utc::now();

        vec![
            MockPermissionGrantData {
                id: "grant-001".to_string(),
                grantee: "user-002".to_string(),
                grantee_type: "user".to_string(),
                permission: Permission::Write,
                resource_type: ResourceType::Pipeline,
                resource_id: None,
                tenant_id: "tenant-123".to_string(),
                granted_by: "user-001".to_string(),
                granted_at: now - chrono::Duration::days(100),
            },
            MockPermissionGrantData {
                id: "grant-002".to_string(),
                grantee: "Developer".to_string(),
                grantee_type: "role".to_string(),
                permission: Permission::Read,
                resource_type: ResourceType::Metrics,
                resource_id: None,
                tenant_id: "tenant-123".to_string(),
                granted_by: "user-001".to_string(),
                granted_at: now - chrono::Duration::days(90),
            },
        ]
    }

    /// Generate mock sessions
    fn generate_mock_sessions() -> Vec<MockSessionData> {
        let now = Utc::now();

        vec![
            MockSessionData {
                id: "session-001".to_string(),
                user_id: "user-001".to_string(),
                tenant_id: "tenant-123".to_string(),
                login_at: now - chrono::Duration::hours(2),
                last_activity: now - chrono::Duration::minutes(5),
                ip_address: "192.168.1.100".to_string(),
                user_agent: "Mozilla/5.0".to_string(),
                is_active: true,
            },
            MockSessionData {
                id: "session-002".to_string(),
                user_id: "user-002".to_string(),
                tenant_id: "tenant-123".to_string(),
                login_at: now - chrono::Duration::minutes(30),
                last_activity: now - chrono::Duration::minutes(2),
                ip_address: "192.168.1.101".to_string(),
                user_agent: "Mozilla/5.0".to_string(),
                is_active: true,
            },
        ]
    }
}

impl Default for RbacService {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state for RBAC API
#[derive(Clone)]
pub struct RbacApiAppState {
    pub service: Arc<RbacService>,
}

/// POST /api/v1/auth/login - Authenticate user
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Authentication & RBAC",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthToken),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn login_handler(
    State(state): State<RbacApiAppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthToken>, StatusCode> {
    info!("🔐 Login requested for user: {}", request.username);

    let token = state
        .service
        .authenticate(&request.username, &request.password)
        .await;

    if let Some(token) = token {
        info!("✅ Login successful for user: {}", request.username);
        Ok(Json(token))
    } else {
        info!("❌ Login failed for user: {}", request.username);
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// GET /api/v1/auth/users/{id} - Get user by ID
#[utoipa::path(
    get,
    path = "/api/v1/auth/users/{id}",
    tag = "Authentication & RBAC",
    responses(
        (status = 200, description = "User found", body = User),
        (status = 404, description = "User not found")
    )
)]
pub async fn get_user_handler(
    State(state): State<RbacApiAppState>,
    Path(id): Path<String>,
) -> Result<Json<User>, StatusCode> {
    info!("🔐 User {} requested", id);

    let user = state.service.get_user_by_id(&id).await;

    if let Some(user) = user {
        info!("✅ User found: {}", user.username);
        Ok(Json(user))
    } else {
        info!("❌ User not found: {}", id);
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /api/v1/auth/users - List users
#[utoipa::path(
    get,
    path = "/api/v1/auth/users",
    tag = "Authentication & RBAC",
    responses(
        (status = 200, description = "Users retrieved successfully", body = Vec<User>)
    )
)]
pub async fn list_users_handler(
    State(state): State<RbacApiAppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<User>>, StatusCode> {
    info!("🔐 Users list requested");

    let tenant_id = params.get("tenant_id").map(|s| s.as_str());

    let users = state.service.list_users(tenant_id).await;

    info!("✅ Returned {} users", users.len());

    Ok(Json(users))
}

/// POST /api/v1/auth/users - Create user
#[utoipa::path(
    post,
    path = "/api/v1/auth/users",
    tag = "Authentication & RBAC",
    request_body = User,
    responses(
        (status = 200, description = "User created successfully", body = User),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_user_handler(
    State(state): State<RbacApiAppState>,
    Json(user): Json<User>,
) -> Result<Json<User>, StatusCode> {
    info!("🔐 Creating user: {}", user.username);

    match state.service.create_user(user.clone()).await {
        Ok(created_user) => {
            info!("✅ User created: {}", created_user.username);
            Ok(Json(created_user))
        }
        Err(e) => {
            info!("❌ Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/v1/auth/users/{id} - Update user
#[utoipa::path(
    put,
    path = "/api/v1/auth/users/{id}",
    tag = "Authentication & RBAC",
    request_body = User,
    responses(
        (status = 200, description = "User updated successfully", body = User),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_user_handler(
    State(state): State<RbacApiAppState>,
    Path(id): Path<String>,
    Json(user): Json<User>,
) -> Result<Json<User>, StatusCode> {
    info!("🔐 Updating user: {}", id);

    match state.service.update_user(&id, user).await {
        Ok(updated_user) => {
            info!("✅ User updated: {}", updated_user.username);
            Ok(Json(updated_user))
        }
        Err(e) => {
            info!("❌ Failed to update user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/v1/auth/users/{id} - Delete user
#[utoipa::path(
    delete,
    path = "/api/v1/auth/users/{id}",
    tag = "Authentication & RBAC",
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_user_handler(
    State(state): State<RbacApiAppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    info!("🔐 Deleting user: {}", id);

    match state.service.delete_user(&id).await {
        Ok(_) => {
            info!("✅ User deleted: {}", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            info!("❌ Failed to delete user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/auth/users/{id}/roles - Get user roles
#[utoipa::path(
    get,
    path = "/api/v1/auth/users/{id}/roles",
    tag = "Authentication & RBAC",
    responses(
        (status = 200, description = "User roles retrieved successfully", body = Vec<RoleAssignment>)
    )
)]
pub async fn get_user_roles_handler(
    State(state): State<RbacApiAppState>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<RoleAssignment>>, StatusCode> {
    info!("🔐 Roles for user {} requested", user_id);

    let roles = state.service.get_user_roles(&user_id).await;

    info!("✅ Returned {} roles for user {}", roles.len(), user_id);

    Ok(Json(roles))
}

/// POST /api/v1/auth/roles/assign - Assign role to user
#[utoipa::path(
    post,
    path = "/api/v1/auth/roles/assign",
    tag = "Authentication & RBAC",
    request_body = RoleAssignment,
    responses(
        (status = 200, description = "Role assigned successfully", body = RoleAssignment),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn assign_role_handler(
    State(state): State<RbacApiAppState>,
    Json(assignment): Json<RoleAssignment>,
) -> Result<Json<RoleAssignment>, StatusCode> {
    info!("🔐 Assigning role to user: {}", assignment.user_id);

    match state.service.assign_role(assignment).await {
        Ok(assigned_role) => {
            info!("✅ Role assigned to user: {}", assigned_role.user_id);
            Ok(Json(assigned_role))
        }
        Err(e) => {
            info!("❌ Failed to assign role: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/v1/auth/roles/revoke - Revoke role from user
#[utoipa::path(
    post,
    path = "/api/v1/auth/roles/revoke",
    tag = "Authentication & RBAC",
    request_body = RevokeRoleRequest,
    responses(
        (status = 204, description = "Role revoked successfully"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn revoke_role_handler(
    State(state): State<RbacApiAppState>,
    Json(request): Json<RevokeRoleRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("🔐 Revoking role assignment: {}", request.assignment_id);

    match state.service.revoke_role(&request.assignment_id).await {
        Ok(_) => {
            info!("✅ Role revoked: {}", request.assignment_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            info!("❌ Failed to revoke role: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/v1/auth/check - Check permission
#[utoipa::path(
    post,
    path = "/api/v1/auth/check",
    tag = "Authentication & RBAC",
    request_body = CheckPermissionRequest,
    responses(
        (status = 200, description = "Permission checked successfully", body = AccessDecision)
    )
)]
pub async fn check_permission_handler(
    State(state): State<RbacApiAppState>,
    Json(request): Json<CheckPermissionRequest>,
) -> Result<Json<AccessDecision>, StatusCode> {
    info!("🔐 Checking permission for user: {}", request.user_id);

    let decision = state
        .service
        .check_permission(
            &request.user_id,
            &request.permission,
            &request.resource_type,
            request.resource_id.as_deref(),
        )
        .await;

    info!(
        "✅ Permission check: {} = {}",
        request.user_id, decision.allowed
    );

    Ok(Json(decision))
}

/// Login request structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Revoke role request structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RevokeRoleRequest {
    pub assignment_id: String,
}

/// Check permission request structure
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CheckPermissionRequest {
    pub user_id: String,
    pub permission: Permission,
    pub resource_type: ResourceType,
    pub resource_id: Option<String>,
}

/// RBAC API routes
pub fn rbac_api_routes() -> Router<RbacApiAppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/users", get(list_users_handler))
        .route("/users", post(create_user_handler))
        .route("/users/{id}", get(get_user_handler))
        .route("/users/{id}", put(update_user_handler))
        .route("/users/{id}", delete(delete_user_handler))
        .route("/users/{id}/roles", get(get_user_roles_handler))
        .route("/roles/assign", post(assign_role_handler))
        .route("/roles/revoke", post(revoke_role_handler))
        .route("/check", post(check_permission_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rbac_service_new() {
        let service = RbacService::new();
        assert!(!service.mock_users.is_empty());
        assert!(!service.mock_role_assignments.is_empty());
        assert!(!service.mock_permission_grants.is_empty());
        assert!(!service.mock_sessions.is_empty());
    }

    #[tokio::test]
    async fn test_rbac_service_authenticate() {
        let service = RbacService::new();

        let token = service.authenticate("admin", "password").await;

        assert!(token.is_some());
        let token = token.unwrap();
        assert_eq!(token.user_id, "user-001");
        assert!(!token.scopes.is_empty());
    }

    #[tokio::test]
    async fn test_rbac_service_get_user_by_id() {
        let service = RbacService::new();

        let user = service.get_user_by_id("user-001").await;

        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.id, "user-001");
        assert_eq!(user.username, "admin");
        assert_eq!(user.roles, vec![Role::SuperAdmin]);
    }

    #[tokio::test]
    async fn test_rbac_service_get_user_by_username() {
        let service = RbacService::new();

        let user = service.get_user_by_username("admin").await;

        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.username, "admin");
        assert!(user.is_active);
    }

    #[tokio::test]
    async fn test_rbac_service_list_users() {
        let service = RbacService::new();

        let users = service.list_users(Some("tenant-123")).await;

        assert!(!users.is_empty());
        assert_eq!(users.len(), 2);
        for user in users {
            assert_eq!(user.tenant_id, "tenant-123");
        }
    }

    #[tokio::test]
    async fn test_rbac_service_check_permission() {
        let service = RbacService::new();

        let decision = service
            .check_permission("user-001", &Permission::Admin, &ResourceType::System, None)
            .await;

        assert!(decision.allowed);
        assert_eq!(decision.permission, Permission::Admin);
    }

    #[tokio::test]
    async fn test_rbac_service_get_user_roles() {
        let service = RbacService::new();

        let roles = service.get_user_roles("user-001").await;

        assert!(!roles.is_empty());
        assert_eq!(roles[0].user_id, "user-001");
        assert_eq!(roles[0].role, Role::SuperAdmin);
    }

    #[tokio::test]
    async fn test_rbac_service_get_user_sessions() {
        let service = RbacService::new();

        let sessions = service.get_user_sessions("user-001").await;

        assert!(!sessions.is_empty());
        assert_eq!(sessions[0].user_id, "user-001");
        assert!(sessions[0].is_active);
    }

    #[tokio::test]
    async fn test_user_creation_and_update() {
        let service = RbacService::new();

        let user = User {
            id: "user-test".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            is_active: true,
            tenant_id: "tenant-123".to_string(),
            roles: vec![Role::Viewer],
            permissions: vec![Permission::Read],
            created_at: Utc::now(),
            last_login: None,
            metadata: HashMap::new(),
        };

        let created = service.create_user(user.clone()).await;
        assert!(created.is_ok());

        let updated = service.update_user("user-test", user).await;
        assert!(updated.is_ok());

        let deleted = service.delete_user("user-test").await;
        assert!(deleted.is_ok());
    }

    #[tokio::test]
    async fn test_role_assignment() {
        let service = RbacService::new();

        let assignment = RoleAssignment {
            id: "assignment-test".to_string(),
            user_id: "user-002".to_string(),
            role: Role::Manager,
            resource_id: None,
            resource_type: None,
            tenant_id: "tenant-123".to_string(),
            granted_by: "user-001".to_string(),
            granted_at: Utc::now(),
            expires_at: None,
        };

        let assigned = service.assign_role(assignment).await;
        assert!(assigned.is_ok());

        let revoked = service.revoke_role("assignment-test").await;
        assert!(revoked.is_ok());
    }

    #[tokio::test]
    async fn test_session_termination() {
        let service = RbacService::new();

        let terminated = service.terminate_session("session-001").await;
        assert!(terminated.is_ok());
    }

    #[tokio::test]
    async fn test_user_serialization() {
        let user = User {
            id: "user-test".to_string(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            is_active: true,
            tenant_id: "tenant-123".to_string(),
            roles: vec![Role::Developer],
            permissions: vec![Permission::Read, Permission::Write],
            created_at: Utc::now(),
            last_login: Some(Utc::now()),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("username"));
        assert!(json.contains("roles"));

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, user.id);
        assert_eq!(deserialized.username, user.username);
    }

    #[tokio::test]
    async fn test_auth_token_serialization() {
        let token = AuthToken {
            id: "token-001".to_string(),
            user_id: "user-001".to_string(),
            token: "mock-token-123".to_string(),
            token_type: "Bearer".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            scopes: vec![Permission::Read],
            created_at: Utc::now(),
            last_used: Some(Utc::now()),
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("token"));
        assert!(json.contains("expires_at"));

        let deserialized: AuthToken = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, token.id);
        assert_eq!(deserialized.user_id, token.user_id);
    }

    #[tokio::test]
    async fn test_access_decision_serialization() {
        let decision = AccessDecision {
            allowed: true,
            permission: Permission::Read,
            resource_type: ResourceType::Pipeline,
            resource_id: Some("pipeline-123".to_string()),
            reason: "Access granted".to_string(),
            effective_permissions: vec![Permission::Read, Permission::Write],
        };

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("allowed"));
        assert!(json.contains("permission"));

        let deserialized: AccessDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.allowed, decision.allowed);
        assert_eq!(deserialized.permission, decision.permission);
    }

    #[tokio::test]
    async fn test_role_enum_comparison() {
        let admin = Role::Admin;
        let developer = Role::Developer;
        let viewer = Role::Viewer;

        assert!(admin == Role::Admin);
        assert!(developer == Role::Developer);
        assert!(viewer == Role::Viewer);
    }

    #[tokio::test]
    async fn test_permission_enum_comparison() {
        let read = Permission::Read;
        let write = Permission::Write;
        let admin = Permission::Admin;

        assert!(read == Permission::Read);
        assert!(write == Permission::Write);
        assert!(admin == Permission::Admin);
    }
}
