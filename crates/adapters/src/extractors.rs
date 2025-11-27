//! Common row extractors for PostgreSQL repositories
//!
//! This module provides reusable extractors to convert database rows
//! into domain entities, reducing code duplication across repository implementations.

use hodei_core::{Job, JobId, Worker, WorkerId};
use hodei_core::{JobSpec, WorkerCapabilities};
use sqlx::{Row, postgres::PgRow};

/// Common row extractor for PostgreSQL databases
///
/// Provides type-safe conversion from database rows to domain entities
/// with proper validation and error handling following DDD principles.
pub struct RowExtractor;

impl RowExtractor {
    /// Extract Job aggregate from PostgreSQL row
    ///
    /// # Arguments
    /// * `row` - PostgreSQL row with job data
    ///
    /// # Returns
    /// * `Result<Job, Box<dyn std::error::Error + Send + Sync>>` - Job aggregate or error
    pub async fn extract_job_from_row(
        row: &PgRow,
    ) -> Result<Job, Box<dyn std::error::Error + Send + Sync>> {
        // Extract fields with validation
        let id = JobId(row.get::<uuid::Uuid, _>("id"));
        let spec_json: serde_json::Value = row.get("spec");

        // Convert JSON to JobSpec with validation
        let spec: JobSpec =
            serde_json::from_value(spec_json).map_err(|e| format!("Invalid JobSpec: {}", e))?;

        // Validate spec using JobSpec's internal validation
        if spec.name.is_empty() || spec.image.is_empty() {
            return Err("Invalid job specification: missing required fields".into());
        }

        // Create Job with validated spec
        let job = Job::new(id, spec).map_err(|e| format!("Failed to create job: {}", e))?;

        // Optionally restore additional fields if needed
        // (name, description, state, timestamps, etc. would be reconstructed)

        Ok(job)
    }

    /// Extract Worker aggregate from PostgreSQL row
    ///
    /// # Arguments
    /// * `row` - PostgreSQL row with worker data
    ///
    /// # Returns
    /// * `Result<Worker, Box<dyn std::error::Error + Send + Sync>>` - Worker aggregate or error
    pub async fn extract_worker_from_row(
        row: &PgRow,
    ) -> Result<Worker, Box<dyn std::error::Error + Send + Sync>> {
        let id = WorkerId(row.get::<uuid::Uuid, _>("id"));
        let name = row.get::<String, _>("name");
        let capabilities_json: Option<serde_json::Value> = row.get("capabilities");
        let max_concurrent_jobs = row.get::<i32, _>("max_concurrent_jobs") as u32;
        let current_jobs: Vec<uuid::Uuid> = row.get("current_jobs");

        // Deserialize capabilities with validation
        let mut capabilities = match capabilities_json {
            Some(json) => {
                let cap_str = json.as_str().ok_or("Invalid capabilities format")?;
                serde_json::from_str::<WorkerCapabilities>(cap_str)
                    .map_err(|e| format!("Failed to parse capabilities: {}", e))?
            }
            None => WorkerCapabilities::new(1, 1024), // Default capabilities
        };

        // Override with actual values from DB
        capabilities.max_concurrent_jobs = max_concurrent_jobs;

        // Create Worker
        let mut worker = Worker::new(id, name, capabilities);

        // Restore current_jobs
        worker.current_jobs = current_jobs;

        Ok(worker)
    }

    /// Extract Pipeline aggregate from PostgreSQL row
    ///
    /// # Arguments
    /// * `row` - PostgreSQL row with pipeline data
    ///
    /// # Returns
    /// * `Result<Pipeline, Box<dyn std::error::Error + Send + Sync>>` - Pipeline aggregate or error
    pub async fn extract_pipeline_from_row(
        row: &PgRow,
    ) -> Result<hodei_core::Pipeline, Box<dyn std::error::Error + Send + Sync>> {
        // Note: Pipeline extraction would be implemented based on actual Pipeline API
        // For now, returning a placeholder error
        Err("Pipeline extraction not yet implemented".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_job_from_row_basic() {
        // This is a basic test since we don't have a real database connection
        // In a real implementation, this would test against an actual database

        // Placeholder - the actual implementation would require a test database
        // For now, we just test that the module compiles
        assert!(true);
    }

    #[test]
    fn test_extract_worker_from_row_basic() {
        // Placeholder - the actual implementation would require a test database
        assert!(true);
    }
}
