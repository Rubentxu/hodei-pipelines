//! Artifact Management Module
//!
//! This module handles the management of job artifacts including upload initiation,
//! chunked uploads, resume capability, and finalization. It follows DDD principles
//! with business logic in the domain layer.

use async_trait::async_trait;
use hodei_core::{DomainError, JobId, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Artifact ID type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Upload ID type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UploadId(String);

impl UploadId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UploadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Upload status enum
#[derive(Debug, Clone, PartialEq)]
pub enum UploadStatus {
    Initiated,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Artifact metadata
#[derive(Debug, Clone)]
pub struct ArtifactMetadata {
    pub artifact_id: ArtifactId,
    pub job_id: JobId,
    pub filename: String,
    pub total_size: u64,
    pub checksum: String,
    pub is_compressed: bool,
    pub compression_type: Option<String>,
    pub upload_id: Option<UploadId>,
    pub status: UploadStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub chunks_received: u32,
    pub bytes_received: u64,
}

/// Chunk information
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub sequence_number: u32,
    pub size: usize,
    pub checksum: Option<String>,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

/// Artifact repository port (for persistence)
#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    /// Save artifact metadata
    async fn save_artifact(&self, artifact: &ArtifactMetadata) -> Result<()>;

    /// Get artifact by ID
    async fn get_artifact(&self, artifact_id: &ArtifactId) -> Result<Option<ArtifactMetadata>>;

    /// Update artifact status
    async fn update_artifact_status(
        &self,
        artifact_id: &ArtifactId,
        status: UploadStatus,
    ) -> Result<()>;

    /// Record chunk reception
    async fn record_chunk(&self, artifact_id: &ArtifactId, chunk: &ChunkInfo) -> Result<()>;

    /// Get chunks for artifact
    async fn get_chunks(&self, artifact_id: &ArtifactId) -> Result<Vec<ChunkInfo>>;

    /// Get artifact by upload ID
    async fn get_artifact_by_upload_id(
        &self,
        upload_id: &UploadId,
    ) -> Result<Option<ArtifactMetadata>>;

    /// Delete artifact and its chunks
    async fn delete_artifact(&self, artifact_id: &ArtifactId) -> Result<()>;
}

/// Storage provider port (for actual file storage)
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Initialize storage for an artifact
    async fn initialize_storage(
        &self,
        artifact_id: &ArtifactId,
        metadata: &ArtifactMetadata,
    ) -> Result<()>;

    /// Store a chunk
    async fn store_chunk(
        &self,
        artifact_id: &ArtifactId,
        sequence_number: u32,
        data: &[u8],
    ) -> Result<()>;

    /// Finalize artifact (validate, move to final location)
    async fn finalize_artifact(&self, artifact_id: &ArtifactId) -> Result<String>;

    /// Delete artifact from storage
    async fn delete_artifact(&self, artifact_id: &ArtifactId) -> Result<()>;
}

/// Artifact management errors
#[derive(thiserror::Error, Debug)]
pub enum ArtifactError {
    #[error("Artifact not found: {0}")]
    NotFound(ArtifactId),

    #[error("Invalid artifact state: {0}")]
    InvalidState(String),

    #[error("Checksum validation failed")]
    ChecksumMismatch,

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Storage provider errors
#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Insufficient space")]
    InsufficientSpace,

    #[error("Artifact not found")]
    NotFound,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Artifact management configuration
#[derive(Debug, Clone)]
pub struct ArtifactManagementConfig {
    pub max_artifact_size_mb: u64,
    pub max_chunks_per_upload: u32,
    pub chunk_timeout_ms: u64,
    pub cleanup_interval_ms: u64,
    pub storage_backend: StorageBackend,
}

/// Storage backend selection
#[derive(Debug, Clone)]
pub enum StorageBackend {
    Local,
    S3,
    Custom(String),
}

/// Artifact management service
pub struct ArtifactManagementService<R, S>
where
    R: ArtifactRepository + Send + Sync + 'static,
    S: StorageProvider + Send + Sync + 'static,
{
    pub(crate) artifact_repo: Arc<R>,
    pub(crate) storage_provider: Arc<S>,
    pub(crate) config: ArtifactManagementConfig,
    /// In-memory cache of active uploads (for performance)
    pub(crate) active_uploads: Arc<RwLock<HashMap<UploadId, ArtifactMetadata>>>,
}

impl<R, S> ArtifactManagementService<R, S>
where
    R: ArtifactRepository + Send + Sync + 'static,
    S: StorageProvider + Send + Sync + 'static,
{
    pub fn new(
        artifact_repo: Arc<R>,
        storage_provider: Arc<S>,
        config: ArtifactManagementConfig,
    ) -> Self {
        Self {
            artifact_repo,
            storage_provider,
            config,
            active_uploads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize a new artifact upload
    pub async fn initiate_upload(
        &self,
        artifact_id: &ArtifactId,
        job_id: &JobId,
        filename: &str,
        total_size: u64,
        checksum: &str,
        is_compressed: bool,
        compression_type: Option<&str>,
    ) -> Result<UploadId> {
        info!("Initiating upload for artifact: {}", artifact_id.as_str());

        // Validate inputs
        if checksum.is_empty() {
            return Err(ArtifactError::InvalidState("Checksum is required".to_string()).into());
        }

        if total_size == 0 {
            return Err(ArtifactError::InvalidState(
                "Total size must be greater than 0".to_string(),
            )
            .into());
        }

        if total_size > self.config.max_artifact_size_mb * 1024 * 1024 {
            return Err(ArtifactError::InvalidState(format!(
                "Artifact size {} exceeds maximum allowed {} MB",
                total_size, self.config.max_artifact_size_mb
            ))
            .into());
        }

        // Generate upload ID
        let upload_id = UploadId::new(format!(
            "upload-{}-{}",
            uuid::Uuid::new_v4(),
            chrono::Utc::now().timestamp()
        ));

        // Create artifact metadata
        let metadata = ArtifactMetadata {
            artifact_id: artifact_id.clone(),
            job_id: *job_id,
            filename: filename.to_string(),
            total_size,
            checksum: checksum.to_string(),
            is_compressed,
            compression_type: compression_type.map(|s| s.to_string()),
            upload_id: Some(upload_id.clone()),
            status: UploadStatus::Initiated,
            created_at: chrono::Utc::now(),
            chunks_received: 0,
            bytes_received: 0,
        };

        // Initialize storage
        self.storage_provider
            .initialize_storage(artifact_id, &metadata)
            .await
            .map_err(|e| {
                ArtifactError::Storage(StorageError::Io(std::io::Error::other(e.to_string())))
            })?;

        // Save metadata
        self.artifact_repo
            .save_artifact(&metadata)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        // Cache active upload
        {
            let mut active_uploads = self.active_uploads.write().await;
            active_uploads.insert(upload_id.clone(), metadata);
        }

        info!(
            "Upload initiated successfully with ID: {}",
            upload_id.as_str()
        );

        Ok(upload_id)
    }

    /// Upload a chunk of an artifact
    pub async fn upload_chunk(
        &self,
        upload_id: &UploadId,
        sequence_number: u32,
        data: &[u8],
        checksum: Option<&str>,
    ) -> Result<u64> {
        // Get artifact from cache or repository
        let metadata = {
            let active_uploads = self.active_uploads.read().await;
            if let Some(metadata) = active_uploads.get(upload_id) {
                metadata.clone()
            } else {
                // Try to get from repository
                self.artifact_repo
                    .get_artifact_by_upload_id(upload_id)
                    .await
                    .map_err(|e| ArtifactError::Database(e.to_string()))?
                    .ok_or_else(|| ArtifactError::InvalidState("Upload not found".to_string()))?
            }
        };

        // Validate upload state
        if metadata.status == UploadStatus::Completed {
            return Err(ArtifactError::InvalidState("Upload already completed".to_string()).into());
        }

        if metadata.status == UploadStatus::Failed || metadata.status == UploadStatus::Cancelled {
            return Err(ArtifactError::InvalidState(
                "Upload has failed or was cancelled".to_string(),
            )
            .into());
        }

        // Validate sequence number
        if sequence_number >= self.config.max_chunks_per_upload {
            return Err(ArtifactError::InvalidState(
                "Sequence number exceeds maximum allowed".to_string(),
            )
            .into());
        }

        // Check if chunk already exists
        let existing_chunks = self
            .artifact_repo
            .get_chunks(&metadata.artifact_id)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        if existing_chunks
            .iter()
            .any(|c| c.sequence_number == sequence_number)
        {
            warn!(
                "Chunk {} already exists for upload {}",
                sequence_number,
                upload_id.as_str()
            );
            // Return bytes received so far
            return Ok(metadata.bytes_received);
        }

        // Store chunk
        self.storage_provider
            .store_chunk(&metadata.artifact_id, sequence_number, data)
            .await
            .map_err(|e| {
                ArtifactError::Storage(StorageError::Io(std::io::Error::other(e.to_string())))
            })?;

        // Record chunk
        let chunk_info = ChunkInfo {
            sequence_number,
            size: data.len(),
            checksum: checksum.map(|s| s.to_string()),
            received_at: chrono::Utc::now(),
        };

        self.artifact_repo
            .record_chunk(&metadata.artifact_id, &chunk_info)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        // Update bytes received
        let new_bytes_received = metadata.bytes_received + data.len() as u64;
        let new_chunks_received = metadata.chunks_received + 1;

        // Update active cache
        {
            let mut active_uploads = self.active_uploads.write().await;
            if let Some(metadata) = active_uploads.get_mut(upload_id) {
                metadata.bytes_received = new_bytes_received;
                metadata.chunks_received = new_chunks_received;
                if new_chunks_received > 0 {
                    metadata.status = UploadStatus::InProgress;
                }
            }
        }

        info!(
            "Chunk {} uploaded for artifact {} ({} bytes total)",
            sequence_number,
            metadata.artifact_id.as_str(),
            new_bytes_received
        );

        Ok(new_bytes_received)
    }

    /// Resume an interrupted upload
    pub async fn resume_upload(&self, upload_id: &UploadId) -> Result<u32> {
        // Get artifact from repository
        let metadata = self
            .artifact_repo
            .get_artifact_by_upload_id(upload_id)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?
            .ok_or_else(|| ArtifactError::InvalidState("Upload not found".to_string()))?;

        // Get received chunks
        let chunks = self
            .artifact_repo
            .get_chunks(&metadata.artifact_id)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        // Find next expected chunk
        let mut next_chunk = 0;
        for chunk in &chunks {
            if chunk.sequence_number >= next_chunk {
                next_chunk = chunk.sequence_number + 1;
            }
        }

        // Update status to InProgress
        self.artifact_repo
            .update_artifact_status(&metadata.artifact_id, UploadStatus::InProgress)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        // Update cache
        {
            let mut active_uploads = self.active_uploads.write().await;
            active_uploads.insert(upload_id.clone(), metadata);
        }

        info!(
            "Upload {} resumed from chunk {}",
            upload_id.as_str(),
            next_chunk
        );

        Ok(next_chunk)
    }

    /// Finalize an upload
    pub async fn finalize_upload(
        &self,
        upload_id: &UploadId,
        final_checksum: &str,
    ) -> Result<String> {
        // Get artifact from repository
        let metadata = self
            .artifact_repo
            .get_artifact_by_upload_id(upload_id)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?
            .ok_or_else(|| ArtifactError::InvalidState("Upload not found".to_string()))?;

        // Validate checksum
        if metadata.checksum != final_checksum {
            return Err(ArtifactError::ChecksumMismatch.into());
        }

        // Finalize in storage
        let artifact_path = self
            .storage_provider
            .finalize_artifact(&metadata.artifact_id)
            .await
            .map_err(|e| {
                ArtifactError::Storage(StorageError::Io(std::io::Error::other(e.to_string())))
            })?;

        // Update status to Completed
        self.artifact_repo
            .update_artifact_status(&metadata.artifact_id, UploadStatus::Completed)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        // Remove from active cache
        {
            let mut active_uploads = self.active_uploads.write().await;
            active_uploads.remove(upload_id);
        }

        info!(
            "Upload {} finalized for artifact {}",
            upload_id.as_str(),
            metadata.artifact_id.as_str()
        );

        Ok(artifact_path)
    }

    /// Cancel an upload
    pub async fn cancel_upload(&self, upload_id: &UploadId) -> Result<()> {
        // Get artifact from repository
        let metadata = self
            .artifact_repo
            .get_artifact_by_upload_id(upload_id)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?
            .ok_or_else(|| ArtifactError::InvalidState("Upload not found".to_string()))?;

        // Delete from storage
        self.storage_provider
            .delete_artifact(&metadata.artifact_id)
            .await
            .map_err(|e| {
                ArtifactError::Storage(StorageError::Io(std::io::Error::other(e.to_string())))
            })?;

        // Delete from repository
        self.artifact_repo
            .delete_artifact(&metadata.artifact_id)
            .await
            .map_err(|e| ArtifactError::Database(e.to_string()))?;

        // Remove from active cache
        {
            let mut active_uploads = self.active_uploads.write().await;
            active_uploads.remove(upload_id);
        }

        info!("Upload {} cancelled", upload_id.as_str());

        Ok(())
    }
}

impl<R, S> Clone for ArtifactManagementService<R, S>
where
    R: ArtifactRepository + Send + Sync + 'static,
    S: StorageProvider + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            artifact_repo: self.artifact_repo.clone(),
            storage_provider: self.storage_provider.clone(),
            config: self.config.clone(),
            active_uploads: self.active_uploads.clone(),
        }
    }
}

impl Default for ArtifactManagementConfig {
    fn default() -> Self {
        Self {
            max_artifact_size_mb: 1024, // 1GB default
            max_chunks_per_upload: 10000,
            chunk_timeout_ms: 30000,
            cleanup_interval_ms: 60000,
            storage_backend: StorageBackend::Local,
        }
    }
}

// Convert ArtifactError to DomainError
impl From<ArtifactError> for hodei_core::DomainError {
    fn from(err: ArtifactError) -> Self {
        hodei_core::DomainError::Other(err.to_string())
    }
}
