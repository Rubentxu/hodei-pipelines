//! PostgreSQL Job Repository
//!
//! Production-ready implementation for persisting and retrieving jobs.

use async_trait::async_trait;
use hodei_pipelines_domain::{DomainError, Job, JobId, Result, WorkerId};
use hodei_pipelines_ports::JobRepository;
use sqlx::{PgPool, Row};
use tracing::info;

/// Default pagination limit for database queries
const DEFAULT_PAGE_SIZE: i64 = 1000;

/// PostgreSQL Job Repository
#[derive(Debug, Clone)]
pub struct PostgreSqlJobRepository {
    pool: PgPool,
    migrations_path: Option<String>,
    migration_file: String,
}

impl PostgreSqlJobRepository {
    /// Create a new PostgreSQL job repository
    pub fn new(pool: PgPool, migrations_path: Option<String>, migration_file: String) -> Self {
        Self {
            pool,
            migrations_path,
            migration_file,
        }
    }

    /// Initialize database schema for jobs
    pub async fn init_schema(&self) -> Result<()> {
        info!("Initializing job schema from migration file");

        // Load migration SQL
        let migration_sql = self.load_migration_sql()?;

        // Parse and execute individual SQL statements
        let statements = Self::parse_sql_statements(&migration_sql)?;

        for stmt in statements {
            sqlx::query(&stmt).execute(&self.pool).await.map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to execute job migration: {}\nStatement: {}",
                    e,
                    stmt.lines().take(3).collect::<Vec<_>>().join(" ")
                ))
            })?;
        }

        info!("Job schema initialized successfully");
        Ok(())
    }

    /// Parse SQL file into individual executable statements
    fn parse_sql_statements(sql: &str) -> Result<Vec<String>> {
        let mut statements = Vec::new();
        let mut current_stmt = String::new();
        let mut chars = sql.chars().peekable();
        let mut in_block_comment = false;
        let mut in_line_comment = false;
        let mut in_dollar_quote = false;
        let mut dollar_quote_tag = String::new();
        let mut paren_depth = 0;

        while let Some(ch) = chars.next() {
            // Handle block comments
            if !in_line_comment && !in_dollar_quote {
                if ch == '/' && chars.peek() == Some(&'*') {
                    in_block_comment = true;
                    chars.next(); // consume '*'
                    continue;
                }
                if ch == '*' && chars.peek() == Some(&'/') {
                    in_block_comment = false;
                    chars.next(); // consume '/'
                    continue;
                }
            }

            // Handle line comments
            if !in_block_comment && !in_dollar_quote
                && ch == '-' && chars.peek() == Some(&'-') {
                    in_line_comment = true;
                    // Skip to end of line
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '\n' {
                            break;
                        }
                        chars.next();
                    }
                    continue;
                }

            // Handle dollar-quoted strings
            if ch == '$' && !in_block_comment && !in_line_comment {
                let mut tag = String::new();
                tag.push('$');

                // Collect the tag
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '$' {
                        tag.push('$');
                        chars.next(); // consume closing $
                        break;
                    } else if next_ch == '\n' {
                        // Invalid dollar quote, treat as regular $
                        break;
                    } else {
                        tag.push(next_ch);
                        chars.next();
                    }
                }

                if !in_dollar_quote {
                    // Start of dollar quote
                    in_dollar_quote = true;
                    dollar_quote_tag = tag.clone();
                    current_stmt.push_str(&tag);
                    continue;
                } else if tag == dollar_quote_tag {
                    // End of dollar quote
                    in_dollar_quote = false;
                    current_stmt.push_str(&tag);
                    dollar_quote_tag.clear();
                    continue;
                }
            }

            if in_block_comment || in_line_comment {
                continue;
            }

            // Track parentheses depth
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')'
                && paren_depth > 0 {
                    paren_depth -= 1;
                }

            // Accumulate character
            current_stmt.push(ch);

            // Check for statement end (semicolon not inside quotes, comments, or parentheses)
            if ch == ';' && !in_dollar_quote && paren_depth == 0 {
                let stmt = current_stmt.trim();
                if !stmt.is_empty() {
                    statements.push(stmt.to_string());
                }
                current_stmt = String::new();
            }
        }

        Ok(statements)
    }

    /// Load migration SQL from file or use embedded fallback
    fn load_migration_sql(&self) -> Result<String> {
        if let Some(custom_path) = &self.migrations_path {
            let path = format!("{}/{}", custom_path, self.migration_file);
            let sql = std::fs::read_to_string(&path).map_err(|e| {
                DomainError::Infrastructure(format!(
                    "Failed to load migration file from custom path {}: {}",
                    path, e
                ))
            })?;
            info!("Loaded job migration from custom path: {}", path);
            return Ok(sql);
        }

        // Use embedded SQL for the configured migration file
        match self.migration_file.as_str() {
            "20241201_jobs.sql" => {
                Ok(include_str!("../../migrations/20241201_jobs.sql").to_string())
            }
            other => Err(DomainError::Infrastructure(format!(
                "Unknown job migration file: {}. Expected one of: 20241201_jobs.sql",
                other
            ))),
        }
    }

    /// Deserialize a Job from a SQL row
    ///
    /// This helper method centralizes the deserialization logic to avoid code duplication
    fn deserialize_job_from_row(&self, row: &sqlx::postgres::PgRow, job_id: JobId) -> Result<Job> {
        let spec: hodei_pipelines_domain::JobSpec =
            serde_json::from_value(row.get("spec")).map_err(|e| {
                DomainError::Validation(format!("Failed to deserialize job spec: {}", e))
            })?;

        let state_str = row.get::<String, _>("state");
        let state = hodei_pipelines_domain::JobState::try_from_str(&state_str)
            .map_err(|_| DomainError::Validation(format!("Invalid job state: {}", state_str)))?;

        Ok(Job {
            id: job_id,
            name: row.get("name"),
            description: row.get("description"),
            spec,
            state,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
            tenant_id: row.get("tenant_id"),
            result: row.get("result"),
        })
    }
}

#[async_trait]
impl JobRepository for PostgreSqlJobRepository {
    async fn save_job(&self, job: &Job) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO jobs (job_id, name, description, spec, state, created_at, updated_at, started_at, completed_at, tenant_id, result)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (job_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                spec = EXCLUDED.spec,
                state = EXCLUDED.state,
                started_at = EXCLUDED.started_at,
                completed_at = EXCLUDED.completed_at,
                tenant_id = EXCLUDED.tenant_id,
                result = EXCLUDED.result,
                updated_at = NOW()
        "#)
        .bind(job.id.as_uuid())
        .bind(&job.name)
        .bind(&job.description)
        .bind(serde_json::to_value(&job.spec).unwrap_or_default())
        .bind(job.state.as_str())
        .bind(job.created_at)
        .bind(job.updated_at)
        .bind(job.started_at)
        .bind(job.completed_at)
        .bind(&job.tenant_id)
        .bind(&job.result)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to save job: {}", e)))?;

        Ok(())
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<Job>> {
        let row = sqlx::query(r#"
            SELECT job_id, name, description, spec, state, created_at, updated_at, started_at, completed_at, tenant_id, result
            FROM jobs
            WHERE job_id = $1
        "#)
        .bind(job_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get job: {}", e)))?;

        if let Some(row) = row {
            Ok(Some(self.deserialize_job_from_row(&row, *job_id)?))
        } else {
            Ok(None)
        }
    }

    async fn get_pending_jobs(&self) -> Result<Vec<Job>> {
        let rows = sqlx::query(r#"
            SELECT job_id, name, description, spec, state, created_at, updated_at, started_at, completed_at, tenant_id, result
            FROM jobs
            WHERE state = 'PENDING'
            ORDER BY created_at
            LIMIT $1
        "#)
        .bind(DEFAULT_PAGE_SIZE)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get pending jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let job_id = JobId::from_uuid(row.get("job_id"));
            jobs.push(self.deserialize_job_from_row(&row, job_id)?);
        }

        Ok(jobs)
    }

    async fn get_running_jobs(&self) -> Result<Vec<Job>> {
        let rows = sqlx::query(r#"
            SELECT job_id, name, description, spec, state, created_at, updated_at, started_at, completed_at, tenant_id, result
            FROM jobs
            WHERE state = 'RUNNING'
            ORDER BY created_at
            LIMIT $1
        "#)
        .bind(DEFAULT_PAGE_SIZE)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to get running jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let job_id = JobId::from_uuid(row.get("job_id"));
            jobs.push(self.deserialize_job_from_row(&row, job_id)?);
        }

        Ok(jobs)
    }

    async fn delete_job(&self, job_id: &JobId) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM jobs WHERE job_id = $1
        "#,
        )
        .bind(job_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to delete job: {}", e)))?;

        Ok(())
    }

    async fn compare_and_swap_status(
        &self,
        job_id: &JobId,
        expected_state: &str,
        new_state: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET state = $1,
                updated_at = NOW()
            WHERE job_id = $2 AND state = $3
        "#,
        )
        .bind(new_state)
        .bind(job_id.as_uuid())
        .bind(expected_state)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to compare and swap status: {}", e))
        })?;

        Ok(result.rows_affected() > 0)
    }

    async fn assign_worker(&self, job_id: &JobId, worker_id: &WorkerId) -> Result<()> {
        // Worker assignment is tracked externally, not in jobs table
        // This is a no-op for now
        let _ = (job_id, worker_id);
        Ok(())
    }

    async fn set_job_start_time(
        &self,
        job_id: &JobId,
        start_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET started_at = $1,
                updated_at = NOW()
            WHERE job_id = $2
        "#,
        )
        .bind(start_time)
        .bind(job_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to set job start time: {}", e)))?;

        Ok(())
    }

    async fn set_job_finish_time(
        &self,
        job_id: &JobId,
        finish_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET completed_at = $1,
                updated_at = NOW()
            WHERE job_id = $2
        "#,
        )
        .bind(finish_time)
        .bind(job_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            DomainError::Infrastructure(format!("Failed to set job finish time: {}", e))
        })?;

        Ok(())
    }

    async fn set_job_duration(&self, job_id: &JobId, duration_ms: i64) -> Result<()> {
        // Duration is calculated from started_at and completed_at
        // This is a no-op for now as we compute it dynamically
        let _ = (job_id, duration_ms);
        Ok(())
    }

    async fn create_job(&self, job_spec: hodei_pipelines_domain::JobSpec) -> Result<JobId> {
        let job_id = JobId::new();
        let job = Job::new(job_id, job_spec)?;
        self.save_job(&job).await?;
        Ok(job_id)
    }

    async fn update_job_state(
        &self,
        job_id: &JobId,
        state: hodei_pipelines_domain::JobState,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE jobs
            SET state = $1,
                updated_at = NOW()
            WHERE job_id = $2
        "#,
        )
        .bind(state.as_str())
        .bind(job_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to update job state: {}", e)))?;

        Ok(())
    }

    async fn list_jobs(&self) -> Result<Vec<Job>> {
        let rows = sqlx::query(r#"
            SELECT job_id, name, description, spec, state, created_at, updated_at, started_at, completed_at, tenant_id, result
            FROM jobs
            ORDER BY created_at DESC
            LIMIT $1
        "#)
        .bind(DEFAULT_PAGE_SIZE)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Infrastructure(format!("Failed to list jobs: {}", e)))?;

        let mut jobs = Vec::new();
        for row in rows {
            let job_id = JobId::from_uuid(row.get("job_id"));
            jobs.push(self.deserialize_job_from_row(&row, job_id)?);
        }

        Ok(jobs)
    }
}
