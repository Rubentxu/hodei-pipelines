use crate::Result;
use crate::config::AuthConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    JobCreate,
    JobRead,
    JobExecute,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Operator,
    Viewer,
}

#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub actor_id: String,
    pub permissions: Vec<Permission>,
}

pub struct AuthorizationService {
    config: AuthConfig,
}

impl AuthorizationService {
    pub fn new(config: AuthConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_service_creation() {
        let config = AuthConfig::default();
        let service = AuthorizationService::new(config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_permissions() {
        assert_eq!(format!("{:?}", Permission::JobCreate), "JobCreate");
        assert_eq!(format!("{:?}", Permission::JobRead), "JobRead");
        assert_eq!(format!("{:?}", Permission::JobExecute), "JobExecute");
    }

    #[test]
    fn test_roles() {
        assert_eq!(format!("{:?}", Role::Admin), "Admin");
        assert_eq!(format!("{:?}", Role::Operator), "Operator");
        assert_eq!(format!("{:?}", Role::Viewer), "Viewer");
    }
}
