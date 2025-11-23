//! Artifact uploader module
//!
//! This module handles uploading job artifacts to the server via gRPC.

use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{error, info};

use super::{CompressionType, Compressor};
use crate::{AgentError, Result};
use anyhow::anyhow;

/// Artifact upload configuration
#[derive(Debug, Clone)]
pub struct ArtifactConfig {
    pub upload_url: String,
    pub compression: CompressionType,
    pub max_file_size_mb: u64,
    pub retry_attempts: u32,
}

impl Default for ArtifactConfig {
    fn default() -> Self {
        Self {
            upload_url: "http://localhost:8080/artifacts".to_string(),
            compression: CompressionType::Gzip,
            max_file_size_mb: 100,
            retry_attempts: 3,
        }
    }
}

/// Artifact uploader
#[derive(Debug)]
pub struct ArtifactUploader {
    config: ArtifactConfig,
    compressor: Compressor,
}

impl ArtifactUploader {
    /// Create a new artifact uploader
    pub fn new(config: ArtifactConfig) -> Self {
        let compressor = Compressor::new(config.compression.clone());
        Self { config, compressor }
    }

    /// Upload a single file
    pub async fn upload_file(&self, file_path: PathBuf) -> Result<String> {
        info!("Uploading artifact: {:?}", file_path);

        // Check file size
        let metadata = fs::metadata(&file_path)
            .await
            .map_err(|e| AgentError::Other(anyhow!("Failed to read file metadata: {}", e)))?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

        if size_mb > self.config.max_file_size_mb as f64 {
            return Err(AgentError::Other(anyhow!(
                "File too large: {:.2} MB",
                size_mb
            )));
        }

        // Read file
        let data = fs::read(&file_path)
            .await
            .map_err(|e| AgentError::Other(anyhow!("Failed to read file: {}", e)))?;

        // Compress if needed
        let (data, _ext) = if matches!(self.config.compression, CompressionType::Gzip) {
            let compressed = self
                .compressor
                .compress(&data)
                .map_err(|e| AgentError::Other(anyhow!("Compression failed: {}", e)))?;
            (compressed, ".gz".to_string())
        } else {
            (data, String::new())
        };

        // TODO: Upload via gRPC
        info!("Artifact upload simulation: {} bytes", data.len());

        // Return URL (simulation)
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        let url = format!("{}/{}", self.config.upload_url, filename);

        Ok(url)
    }

    /// Upload multiple files
    pub async fn upload_files(&self, files: Vec<PathBuf>) -> Result<Vec<String>> {
        info!("Uploading {} artifacts", files.len());

        let mut urls = Vec::new();

        for file in files {
            match self.upload_file(file).await {
                Ok(url) => urls.push(url),
                Err(e) => {
                    error!("Failed to upload artifact: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(urls)
    }

    /// Upload directory recursively
    pub async fn upload_directory(&self, dir_path: PathBuf) -> Result<Vec<String>> {
        info!("Uploading directory: {:?}", dir_path);

        let mut files = Vec::new();

        // Collect all files iteratively
        let mut stack = vec![dir_path];

        while let Some(current_dir) = stack.pop() {
            let mut entries = fs::read_dir(&current_dir)
                .await
                .map_err(|e| AgentError::Other(anyhow!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| AgentError::Other(anyhow!("Failed to read directory entry: {}", e)))?
            {
                let path = entry.path();

                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() {
                    stack.push(path);
                }
            }
        }

        self.upload_files(files).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_artifact_uploader_creation() {
        let config = ArtifactConfig::default();
        let uploader = ArtifactUploader::new(config);
        assert!(uploader.config.retry_attempts > 0);
    }

    #[tokio::test]
    async fn test_upload_single_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test data").unwrap();

        let config = ArtifactConfig::default();
        let uploader = ArtifactUploader::new(config);

        let result = uploader.upload_file(temp_file.path().to_path_buf()).await;
        assert!(result.is_ok());
    }
}
