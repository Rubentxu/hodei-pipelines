//! PostgreSQL Repository Implementations
//!
//! Concrete implementations of repository ports using SQLx and PostgreSQL

use domain::job_execution::{Job, JobRepository, JobSpec};
use domain::provider_management::{Provider, ProviderRepository};
use domain::provider_management::entities::ProviderStatus;
use domain::shared_kernel::{DomainError, DomainResult, JobId, ProviderId, ProviderType, JobState};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use tracing::info;

/// PostgreSQL connection pool configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: Option<u32>,
    pub idle_timeout: Option<std::time::Duration>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://hodei:hodei_password@localhost:5432/hodei_jobs".to_string(),
            max_connections: 10,
            min_connections: Some(1),
            idle_timeout: Some(std::time::Duration::from_secs(600)),
        }
    }
}

/// PostgreSQL connection pool manager
pub struct DatabasePool {
    pool: Pool<Postgres>,
}

impl DatabasePool {
    /// Create a new database pool from configuration
    pub async fn new(config: DatabaseConfig) -> DomainResult<Self> {
        info!("Connecting to PostgreSQL database...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections.unwrap_or(1))
            .idle_timeout(config.idle_timeout)
            .connect(&config.url)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to connect to database: {}", e)))?;

        info!("Successfully connected to PostgreSQL database");
        Ok(Self { pool })
    }

    /// Get the underlying pool reference
    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }

    /// Execute a health check on the database
    pub async fn health_check(&self) -> DomainResult<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Database health check failed: {}", e)))?;

        Ok(())
    }
}

/// PostgreSQL implementation of JobRepository
pub struct PostgresJobRepository {
    pool: Pool<Postgres>,
}

impl PostgresJobRepository {
    /// Create a new PostgreSQL job repository
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Convert JobState to database string representation
    fn job_state_to_db(state: &JobState) -> &'static str {
        match state {
            JobState::Pending => "pending",
            JobState::Running => "running",
            JobState::Completed => "completed",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
        }
    }

    /// Convert database string to JobState
    fn db_to_job_state(state: &str) -> JobState {
        match state {
            "pending" => JobState::Pending,
            "running" => JobState::Running,
            "completed" => JobState::Completed,
            "failed" => JobState::Failed,
            "cancelled" => JobState::Cancelled,
            _ => JobState::Pending,
        }
    }
}

#[async_trait::async_trait]
impl JobRepository for PostgresJobRepository {
    async fn save(&self, job: &Job) -> DomainResult<()> {
        let state = Self::job_state_to_db(&job.state);

        sqlx::query(
            r#"
            INSERT INTO jobs (id, job_spec, state, completed_at, error_message)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                job_spec = EXCLUDED.job_spec,
                state = EXCLUDED.state,
                completed_at = EXCLUDED.completed_at,
                error_message = EXCLUDED.error_message
            "#
        )
        .bind(&job.id.0)
        .bind(serde_json::to_value(&job.spec).map_err(|e| DomainError::Infrastructure(format!("Failed to serialize job spec: {}", e)))?)
        .bind(state)
        .bind(job.completed_at.map(|dt| dt.naive_utc()))
        .bind(job.error_message.as_ref())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save job: {}", e)))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &JobId) -> DomainResult<Option<Job>> {
        let row = sqlx::query(
            r#"
            SELECT id, job_spec, state, completed_at, error_message
            FROM jobs
            WHERE id = $1
            "#
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to find job: {}", e)))?;

        if let Some(row) = row {
            let job_id: String = row.get("id");
            let job_spec: serde_json::Value = row.get("job_spec");
            let state: String = row.get("state");
            let completed_at: Option<chrono::NaiveDateTime> = row.get("completed_at");
            let error_message: Option<String> = row.get("error_message");

            let spec: JobSpec = serde_json::from_value(job_spec)
                .map_err(|e| DomainError::Infrastructure(format!("Failed to deserialize job spec: {}", e)))?;

            let job = Job {
                id: JobId(job_id),
                spec,
                state: Self::db_to_job_state(&state),
                execution_context: None,
                completed_at: completed_at.map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
                error_message,
            };

            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn list(&self) -> DomainResult<Vec<Job>> {
        let rows = sqlx::query(
            r#"
            SELECT id, job_spec, state, completed_at, error_message
            FROM jobs
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to list jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let job_id: String = row.get("id");
            let job_spec: serde_json::Value = row.get("job_spec");
            let state: String = row.get("state");
            let completed_at: Option<chrono::NaiveDateTime> = row.get("completed_at");
            let error_message: Option<String> = row.get("error_message");

            let spec: JobSpec = serde_json::from_value(job_spec)
                .map_err(|e| DomainError::Infrastructure(format!("Failed to deserialize job spec: {}", e)))?;

            let job = Job {
                id: JobId(job_id),
                spec,
                state: Self::db_to_job_state(&state),
                execution_context: None,
                completed_at: completed_at.map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
                error_message,
            };

            jobs.push(job);
        }

        Ok(jobs)
    }

    async fn delete(&self, id: &JobId) -> DomainResult<()> {
        sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to delete job: {}", e)))?;

        Ok(())
    }
}

/// PostgreSQL implementation of ProviderRepository
pub struct PostgresProviderRepository {
    pool: Pool<Postgres>,
}

impl PostgresProviderRepository {
    /// Create a new PostgreSQL provider repository
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Convert ProviderType to database string representation
    fn provider_type_to_db(provider_type: &ProviderType) -> &'static str {
        match provider_type {
            ProviderType::Docker => "docker",
            ProviderType::Kubernetes => "kubernetes",
            ProviderType::Lambda => "lambda",
            ProviderType::AzureVM => "azure_vm",
            ProviderType::GCPFunctions => "gcp_functions",
        }
    }

    /// Convert database string to ProviderType
    fn db_to_provider_type(provider_type: &str) -> ProviderType {
        match provider_type {
            "docker" => ProviderType::Docker,
            "kubernetes" => ProviderType::Kubernetes,
            "lambda" => ProviderType::Lambda,
            "azure_vm" => ProviderType::AzureVM,
            "gcp_functions" => ProviderType::GCPFunctions,
            _ => ProviderType::Docker,
        }
    }
}

#[async_trait::async_trait]
impl ProviderRepository for PostgresProviderRepository {
    async fn save(&self, provider: &Provider) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO providers (id, name, provider_type, status, capabilities, config)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                provider_type = EXCLUDED.provider_type,
                status = EXCLUDED.status,
                capabilities = EXCLUDED.capabilities,
                config = EXCLUDED.config
            "#
        )
        .bind(&provider.id.0)
        .bind(&provider.name)
        .bind(Self::provider_type_to_db(&provider.provider_type))
        .bind(format!("{:?}", provider.status).to_lowercase())
        .bind(serde_json::to_value(&provider.capabilities).map_err(|e| DomainError::Infrastructure(format!("Failed to serialize capabilities: {}", e)))?)
        .bind(serde_json::to_value(&provider.config).map_err(|e| DomainError::Infrastructure(format!("Failed to serialize config: {}", e)))?)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save provider: {}", e)))?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ProviderId) -> DomainResult<Option<Provider>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, provider_type, status, capabilities, config
            FROM providers
            WHERE id = $1
            "#
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to find provider: {}", e)))?;

        if let Some(row) = row {
            let provider_id: String = row.get("id");
            let name: String = row.get("name");
            let provider_type: String = row.get("provider_type");
            let status: String = row.get("status");
            let capabilities: serde_json::Value = row.get("capabilities");
            let config: serde_json::Value = row.get("config");

            let capabilities_deserialized = serde_json::from_value(capabilities)
                .map_err(|e| DomainError::Infrastructure(format!("Failed to deserialize capabilities: {}", e)))?;

            let config_deserialized = serde_json::from_value(config)
                .map_err(|e| DomainError::Infrastructure(format!("Failed to deserialize config: {}", e)))?;

            let provider = Provider {
                id: ProviderId(provider_id),
                name,
                provider_type: Self::db_to_provider_type(&provider_type),
                status: match status.as_str() {
                    "active" => ProviderStatus::Active,
                    "inactive" => ProviderStatus::Inactive,
                    "error" => ProviderStatus::Error,
                    _ => ProviderStatus::Inactive,
                },
                capabilities: capabilities_deserialized,
                config: config_deserialized,
            };

            Ok(Some(provider))
        } else {
            Ok(None)
        }
    }

    async fn list(&self) -> DomainResult<Vec<Provider>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, provider_type, status, capabilities, config
            FROM providers
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to find all providers: {}", e)))?;

        let mut providers = Vec::new();
        for row in rows {
            let provider_id: String = row.get("id");
            let name: String = row.get("name");
            let provider_type: String = row.get("provider_type");
            let status: String = row.get("status");
            let capabilities: serde_json::Value = row.get("capabilities");
            let config: serde_json::Value = row.get("config");

            let capabilities_deserialized = serde_json::from_value(capabilities)
                .map_err(|e| DomainError::Infrastructure(format!("Failed to deserialize capabilities: {}", e)))?;

            let config_deserialized = serde_json::from_value(config)
                .map_err(|e| DomainError::Infrastructure(format!("Failed to deserialize config: {}", e)))?;

            let provider = Provider {
                id: ProviderId(provider_id),
                name,
                provider_type: Self::db_to_provider_type(&provider_type),
                status: match status.as_str() {
                    "active" => ProviderStatus::Active,
                    "inactive" => ProviderStatus::Inactive,
                    "error" => ProviderStatus::Error,
                    _ => ProviderStatus::Inactive,
                },
                capabilities: capabilities_deserialized,
                config: config_deserialized,
            };

            providers.push(provider);
        }

        Ok(providers)
    }

    async fn delete(&self, id: &ProviderId) -> DomainResult<()> {
        sqlx::query("DELETE FROM providers WHERE id = $1")
            .bind(&id.0)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Infrastructure(format!("Failed to delete provider: {}", e)))?;

        Ok(())
    }
}
