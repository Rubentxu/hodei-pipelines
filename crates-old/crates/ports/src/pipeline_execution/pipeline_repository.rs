//! Pipeline Repository Port

use async_trait::async_trait;
use hodei_pipelines_domain::{Pipeline, PipelineId, Result};

#[cfg_attr(feature = "testing", mockall::automock)]
#[async_trait]
pub trait PipelineRepository: Send + Sync {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<()>;
    async fn get_pipeline(&self, id: &PipelineId) -> Result<Option<Pipeline>>;
    async fn delete_pipeline(&self, id: &PipelineId) -> Result<()>;
    async fn get_all_pipelines(&self) -> Result<Vec<Pipeline>>;
}

#[derive(thiserror::Error, Debug)]
pub enum PipelineRepositoryError {
    #[error("Pipeline not found: {0}")]
    NotFound(PipelineId),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Validation error: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_repository_trait_exists() {
        // This test verifies the trait exists and compiles
        let _repo: Option<Box<dyn PipelineRepository + Send + Sync>> = None;
        // Trait exists and compiles correctly
    }

    #[test]
    fn test_pipeline_repository_error_constructors() {
        // Test error constructors
        let _not_found = PipelineRepositoryError::Database("error".to_string());
        let _database = PipelineRepositoryError::Database("error".to_string());
    }

    #[test]
    fn test_pipeline_repository_error_display() {
        let database = PipelineRepositoryError::Database("Connection error".to_string());
        assert!(database.to_string().contains("Database error"));
    }
}
