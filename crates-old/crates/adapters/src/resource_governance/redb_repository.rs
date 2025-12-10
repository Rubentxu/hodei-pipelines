//! Redb Resource Pool Repository
//!
//! Implementation of ResourcePoolRepository using Redb.

use async_trait::async_trait;
use hodei_pipelines_domain::DomainError;
use hodei_pipelines_domain::resource_governance::{ResourcePoolConfig, ResourcePoolRepository};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Redb Resource Pool Repository
#[derive(Debug, Clone)]
pub struct RedbResourcePoolRepository {
    db: Arc<Mutex<Database>>,
}

impl RedbResourcePoolRepository {
    /// Table definition for resource pools - Key: pool_name (String), Value: ResourcePoolConfig (JSON)
    const POOLS_TABLE: TableDefinition<'_, &str, Vec<u8>> = TableDefinition::new("resource_pools");

    /// Create a new RedbResourcePoolRepository with in-memory database
    pub fn new_in_memory() -> Result<Self, DomainError> {
        let db = Arc::new(Mutex::new(Database::create("mem:redb-pools").map_err(
            |e| DomainError::Infrastructure(format!("Failed to create Redb database: {}", e)),
        )?));

        Ok(Self { db })
    }

    /// Create a new RedbResourcePoolRepository with database file
    pub fn new_with_path(path: &str) -> Result<Self, DomainError> {
        let db = Arc::new(Mutex::new(Database::create(path).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create Redb database: {}", e))
        })?));

        Ok(Self { db })
    }

    /// Create a new RedbResourcePoolRepository from an existing shared database instance
    pub fn new_from_db(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<(), DomainError> {
        info!("Initializing Redb schema for resource pools");

        let db = self.db.lock().await;
        let tx = db.begin_write().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to begin transaction: {}", e))
        })?;

        // Create pools table
        tx.open_table(Self::POOLS_TABLE).map_err(|e| {
            DomainError::Infrastructure(format!("Failed to create resource_pools table: {}", e))
        })?;

        tx.commit().map_err(|e| {
            DomainError::Infrastructure(format!("Failed to commit transaction: {}", e))
        })?;

        info!("Redb schema for resource pools initialized successfully");
        Ok(())
    }

    /// Serialize config to bytes using JSON
    fn config_to_bytes(config: &ResourcePoolConfig) -> Vec<u8> {
        serde_json::to_vec(config).expect("Failed to serialize ResourcePoolConfig")
    }

    /// Deserialize config from bytes using JSON
    fn bytes_to_config(data: &[u8]) -> Option<ResourcePoolConfig> {
        serde_json::from_slice(data).ok()
    }
}

#[async_trait]
impl ResourcePoolRepository for RedbResourcePoolRepository {
    async fn save(&self, config: ResourcePoolConfig) -> Result<(), String> {
        debug!("Saving resource pool: {}", config.name);

        let db = self.db.lock().await;
        let tx = db
            .begin_write()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let mut table = tx
            .open_table(Self::POOLS_TABLE)
            .map_err(|e| format!("Failed to open table: {}", e))?;

        let value = Self::config_to_bytes(&config);
        table
            .insert(config.name.as_str(), value)
            .map_err(|e| format!("Failed to insert pool: {}", e))?;

        drop(table);
        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    async fn get(&self, name: &str) -> Result<Option<ResourcePoolConfig>, String> {
        let db = self.db.lock().await;
        let tx = db
            .begin_read()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let table = tx
            .open_table(Self::POOLS_TABLE)
            .map_err(|e| format!("Failed to open table: {}", e))?;

        let value = table
            .get(name)
            .map_err(|e| format!("Failed to get pool: {}", e))?;

        if let Some(val) = value {
            let config = Self::bytes_to_config(&val.value())
                .ok_or_else(|| "Failed to deserialize ResourcePoolConfig".to_string())?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    async fn list(&self) -> Result<Vec<ResourcePoolConfig>, String> {
        let db = self.db.lock().await;
        let tx = db
            .begin_read()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let table = tx
            .open_table(Self::POOLS_TABLE)
            .map_err(|e| format!("Failed to open table: {}", e))?;

        let mut pools = Vec::new();
        let iter = table
            .iter()
            .map_err(|e| format!("Failed to iterate table: {}", e))?;

        for item in iter {
            let (_key, value) = item.map_err(|e| format!("Failed to read item: {}", e))?;
            if let Some(config) = Self::bytes_to_config(&value.value()) {
                pools.push(config);
            }
        }

        Ok(pools)
    }

    async fn delete(&self, name: &str) -> Result<(), String> {
        debug!("Deleting resource pool: {}", name);

        let db = self.db.lock().await;
        let tx = db
            .begin_write()
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        let mut table = tx
            .open_table(Self::POOLS_TABLE)
            .map_err(|e| format!("Failed to open table: {}", e))?;

        table
            .remove(name)
            .map_err(|e| format!("Failed to remove pool: {}", e))?;

        drop(table);
        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hodei_pipelines_domain::ResourceQuota;
    use hodei_pipelines_domain::resource_governance::{ProviderType, ResourcePoolConfig};
    use hodei_pipelines_ports::ResourcePoolRepository;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_redb_resource_pool_crud() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_pools.redb");
        let repo = super::RedbResourcePoolRepository::new_with_path(db_path.to_str().unwrap()).unwrap();

        repo.init_schema().await.unwrap();

        let config = ResourcePoolConfig {
            provider_type: ProviderType::Docker,
            name: "test-pool".to_string(),
            provider_name: "local-docker".to_string(),
            min_size: 1,
            max_size: 5,
            default_resources: ResourceQuota::default(),
            tags: HashMap::new(),
        };

        // Save
        repo.save(config.clone()).await.unwrap();

        // Get
        let retrieved = repo.get("test-pool").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), config);

        // List
        let list = repo.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], config);

        // Delete
        repo.delete("test-pool").await.unwrap();
        let retrieved_after_delete = repo.get("test-pool").await.unwrap();
        assert!(retrieved_after_delete.is_none());
    }
}
