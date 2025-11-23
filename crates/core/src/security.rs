use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Operator,
    Viewer,
    Worker,
    System,
}

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

#[derive(Debug, Clone)]
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
