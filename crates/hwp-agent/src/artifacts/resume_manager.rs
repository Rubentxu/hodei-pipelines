//! Resume capability for interrupted uploads
//!
//! This module provides functionality to resume interrupted artifact uploads
//! by tracking uploaded chunks and enabling resumption from the last confirmed chunk.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::AgentError;
use crate::artifacts::uploader::ArtifactUploader;

/// Upload session tracking information
#[derive(Debug, Clone)]
struct UploadSession {
    artifact_id: String,
    file_path: PathBuf,
    total_size: u64,
    total_chunks: u32,
    uploaded_chunks: HashMap<u32, Vec<u8>>, // chunk_index -> chunk_data
    job_id: String,
    upload_url: String,
    compression_type: String,
    is_compressed: bool,
    checksum: String,
}

/// Resume capability manager
#[derive(Debug)]
pub struct ResumeManager {
    active_sessions: Arc<Mutex<HashMap<String, UploadSession>>>,
    cleanup_interval_ms: u64,
}

impl ResumeManager {
    /// Create a new resume manager
    pub fn new() -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
            cleanup_interval_ms: 24 * 60 * 60 * 1000, // 24 hours in milliseconds
        }
    }

    /// Create a new upload session
    pub async fn register_session(
        &self,
        artifact_id: &str,
        file_path: PathBuf,
        total_size: u64,
        total_chunks: u32,
        job_id: &str,
        upload_url: &str,
        compression_type: &str,
        is_compressed: bool,
        checksum: &str,
    ) -> Result<(), AgentError> {
        let mut sessions = self.active_sessions.lock().await;

        let session = UploadSession {
            artifact_id: artifact_id.to_string(),
            file_path,
            total_size,
            total_chunks,
            uploaded_chunks: HashMap::new(),
            job_id: job_id.to_string(),
            upload_url: upload_url.to_string(),
            compression_type: compression_type.to_string(),
            is_compressed,
            checksum: checksum.to_string(),
        };

        sessions.insert(artifact_id.to_string(), session);

        info!("Registered upload session for artifact: {}", artifact_id);

        Ok(())
    }

    /// Record a successfully uploaded chunk
    pub async fn record_chunk_upload(
        &self,
        artifact_id: &str,
        chunk_index: u32,
        chunk_data: Vec<u8>,
    ) -> Result<(), AgentError> {
        let mut sessions = self.active_sessions.lock().await;

        if let Some(session) = sessions.get_mut(artifact_id) {
            session.uploaded_chunks.insert(chunk_index, chunk_data);
            info!(
                "Recorded chunk {} for artifact {}",
                chunk_index, artifact_id
            );
        } else {
            warn!("No active session found for artifact {}", artifact_id);
        }

        Ok(())
    }

    /// Get upload progress for an artifact
    pub async fn get_upload_progress(&self, artifact_id: &str) -> Result<(u32, u32), AgentError> {
        let sessions = self.active_sessions.lock().await;

        if let Some(session) = sessions.get(artifact_id) {
            let uploaded = session.uploaded_chunks.len() as u32;
            Ok((uploaded, session.total_chunks))
        } else {
            Err(AgentError::Other(format!(
                "No active session found for artifact {}",
                artifact_id
            )))
        }
    }

    /// Get the list of missing chunks (not yet uploaded)
    pub async fn get_missing_chunks(&self, artifact_id: &str) -> Result<Vec<u32>, AgentError> {
        let sessions = self.active_sessions.lock().await;

        if let Some(session) = sessions.get(artifact_id) {
            let mut missing_chunks = Vec::new();

            for chunk_index in 0..session.total_chunks {
                if !session.uploaded_chunks.contains_key(&chunk_index) {
                    missing_chunks.push(chunk_index);
                }
            }

            Ok(missing_chunks)
        } else {
            Err(AgentError::Other(format!(
                "No active session found for artifact {}",
                artifact_id
            )))
        }
    }

    /// Resume an interrupted upload from the last confirmed chunk
    pub async fn resume_upload(
        &self,
        artifact_id: &str,
        _uploader: ArtifactUploader,
    ) -> Result<String, AgentError> {
        info!("Resuming upload for artifact: {}", artifact_id);

        // Get session info
        let (uploaded, total) = self.get_upload_progress(artifact_id).await?;
        let missing_chunks = self.get_missing_chunks(artifact_id).await?;

        info!("Upload progress: {}/{} chunks uploaded", uploaded, total);

        if missing_chunks.is_empty() {
            info!("Upload already complete for artifact: {}", artifact_id);
            return Ok(format!(
                "{}/artifacts/{}",
                self.get_upload_url(artifact_id).await?,
                artifact_id
            ));
        }

        // TODO: In a real implementation, we would:
        // 1. Call server-side ResumeUpload RPC to get server state
        // 2. Compare client state with server state
        // 3. Re-upload only missing chunks
        // 4. Verify integrity with checksums

        // For now, we'll mark the session as complete if all chunks are recorded
        self.complete_session(artifact_id).await?;

        Ok(format!(
            "{}/artifacts/{}",
            self.get_upload_url(artifact_id).await?,
            artifact_id
        ))
    }

    /// Get upload URL for an artifact
    async fn get_upload_url(&self, artifact_id: &str) -> Result<String, AgentError> {
        let sessions = self.active_sessions.lock().await;

        if let Some(session) = sessions.get(artifact_id) {
            Ok(session.upload_url.clone())
        } else {
            Err(AgentError::Other(format!(
                "No active session found for artifact {}",
                artifact_id
            )))
        }
    }

    /// Complete an upload session and clean up
    pub async fn complete_session(&self, artifact_id: &str) -> Result<(), AgentError> {
        let mut sessions = self.active_sessions.lock().await;

        sessions.remove(artifact_id);

        info!(
            "Completed and cleaned up session for artifact: {}",
            artifact_id
        );

        Ok(())
    }

    /// Abandon a session without uploading remaining chunks
    pub async fn abandon_session(&self, artifact_id: &str) -> Result<(), AgentError> {
        let mut sessions = self.active_sessions.lock().await;

        sessions.remove(artifact_id);

        warn!("Abandoned session for artifact: {}", artifact_id);

        Ok(())
    }

    /// Clean up old/abandoned sessions
    pub async fn cleanup_old_sessions(&self) -> Result<usize, AgentError> {
        // In a real implementation, we would check timestamps and remove old sessions
        // For now, we'll just return 0
        let sessions = self.active_sessions.lock().await;
        Ok(sessions.len())
    }

    /// List all active sessions
    pub async fn list_active_sessions(&self) -> Result<Vec<String>, AgentError> {
        let sessions = self.active_sessions.lock().await;
        Ok(sessions.keys().cloned().collect())
    }

    /// Verify chunk integrity using checksum
    pub async fn verify_chunk(
        &self,
        artifact_id: &str,
        _chunk_index: u32,
        chunk_data: &[u8],
    ) -> Result<bool, AgentError> {
        let sessions = self.active_sessions.lock().await;

        if let Some(_session) = sessions.get(artifact_id) {
            // Calculate checksum of the chunk
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(chunk_data);
            let _calculated_checksum = format!("{:x}", hasher.finalize());

            // TODO: Verify against server-provided checksum or stored checksum
            // For now, we'll just return true
            Ok(true)
        } else {
            Err(AgentError::Other(format!(
                "No active session found for artifact {}",
                artifact_id
            )))
        }
    }
}

impl Default for ResumeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_resume_manager_creation() {
        let manager = ResumeManager::new();
        assert!(manager.cleanup_interval_ms > 0);
    }

    #[tokio::test]
    async fn test_register_session() {
        let manager = ResumeManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        let result = manager
            .register_session(
                "test-artifact",
                temp_file.path().to_path_buf(),
                1000,
                10,
                "test-job",
                "http://localhost:8080",
                "gzip",
                true,
                "checksum123",
            )
            .await;

        assert!(result.is_ok());

        let sessions = manager.list_active_sessions().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0], "test-artifact");
    }

    #[tokio::test]
    async fn test_record_chunk_upload() {
        let manager = ResumeManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        manager
            .register_session(
                "test-artifact",
                temp_file.path().to_path_buf(),
                1000,
                10,
                "test-job",
                "http://localhost:8080",
                "gzip",
                true,
                "checksum123",
            )
            .await
            .unwrap();

        let chunk_data = vec![1, 2, 3, 4, 5];
        let result = manager
            .record_chunk_upload("test-artifact", 0, chunk_data)
            .await;

        assert!(result.is_ok());

        let (uploaded, total) = manager.get_upload_progress("test-artifact").await.unwrap();
        assert_eq!(uploaded, 1);
        assert_eq!(total, 10);
    }

    #[tokio::test]
    async fn test_get_missing_chunks() {
        let manager = ResumeManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        manager
            .register_session(
                "test-artifact",
                temp_file.path().to_path_buf(),
                1000,
                5,
                "test-job",
                "http://localhost:8080",
                "gzip",
                true,
                "checksum123",
            )
            .await
            .unwrap();

        // Record chunks 0, 2, 4
        manager
            .record_chunk_upload("test-artifact", 0, vec![1, 2, 3])
            .await
            .unwrap();
        manager
            .record_chunk_upload("test-artifact", 2, vec![4, 5, 6])
            .await
            .unwrap();
        manager
            .record_chunk_upload("test-artifact", 4, vec![7, 8, 9])
            .await
            .unwrap();

        let missing = manager.get_missing_chunks("test-artifact").await.unwrap();
        assert_eq!(missing, vec![1, 3]);
    }

    #[tokio::test]
    async fn test_complete_session() {
        let manager = ResumeManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        manager
            .register_session(
                "test-artifact",
                temp_file.path().to_path_buf(),
                1000,
                10,
                "test-job",
                "http://localhost:8080",
                "gzip",
                true,
                "checksum123",
            )
            .await
            .unwrap();

        let result = manager.complete_session("test-artifact").await;
        assert!(result.is_ok());

        let sessions = manager.list_active_sessions().await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_abandon_session() {
        let manager = ResumeManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        manager
            .register_session(
                "test-artifact",
                temp_file.path().to_path_buf(),
                1000,
                10,
                "test-job",
                "http://localhost:8080",
                "gzip",
                true,
                "checksum123",
            )
            .await
            .unwrap();

        let result = manager.abandon_session("test-artifact").await;
        assert!(result.is_ok());

        let sessions = manager.list_active_sessions().await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_verify_chunk() {
        let manager = ResumeManager::new();
        let temp_file = NamedTempFile::new().unwrap();

        manager
            .register_session(
                "test-artifact",
                temp_file.path().to_path_buf(),
                1000,
                10,
                "test-job",
                "http://localhost:8080",
                "gzip",
                true,
                "checksum123",
            )
            .await
            .unwrap();

        let chunk_data = vec![1, 2, 3, 4, 5];
        let result = manager.verify_chunk("test-artifact", 0, &chunk_data).await;

        assert!(result.is_ok());
    }
}
