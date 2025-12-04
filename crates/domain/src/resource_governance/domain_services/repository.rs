use crate::resource_governance::ResourcePoolConfig;
use async_trait::async_trait;

/// Repository for persisting resource pools
#[async_trait]
pub trait ResourcePoolRepository: Send + Sync {
    /// Save or update a resource pool configuration
    async fn save(&self, config: ResourcePoolConfig) -> Result<(), String>;

    /// Get a resource pool configuration by name
    async fn get(&self, name: &str) -> Result<Option<ResourcePoolConfig>, String>;

    /// List all resource pool configurations
    async fn list(&self) -> Result<Vec<ResourcePoolConfig>, String>;

    /// Delete a resource pool configuration
    async fn delete(&self, name: &str) -> Result<(), String>;
}
