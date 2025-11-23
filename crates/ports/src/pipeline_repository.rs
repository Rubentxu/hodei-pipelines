//! Pipeline Repository Port

use async_trait::async_trait;
use hodei_core::{Pipeline, PipelineId};

#[async_trait]
pub trait PipelineRepository: Send + Sync {
    async fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), PipelineRepositoryError>;
    async fn get_pipeline(&self, id: &PipelineId) -> Result<Option<Pipeline>, PipelineRepositoryError>;
    async fn delete_pipeline(&self, id: &PipelineId) -> Result<(), PipelineRepositoryError>;
}

#[derive(thiserror::Error, Debug)]
pub enum PipelineRepositoryError {
    #[error("Pipeline not found: {0}")]
    NotFound(PipelineId),
    #[error("Database error: {0}")]
    Database(String),
}
