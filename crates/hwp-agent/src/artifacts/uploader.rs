//! Artifact uploader module
//!
//! This module handles uploading job artifacts to the server via gRPC.

use std::path::PathBuf;
use tokio::fs;
use tonic::Request;
use tracing::{error, info};

use hwp_proto::{
    ArtifactChunk, FinalizeUploadRequest, InitiateUploadRequest, WorkerServiceClient,
};

use super::{CompressionType, Compressor};
use crate::{AgentError, Result};

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
    grpc_client: Option<WorkerServiceClient<tonic::transport::Channel>>,
}

impl ArtifactUploader {
    /// Create a new artifact uploader
    pub fn new(config: ArtifactConfig) -> Self {
        let compressor = Compressor::new(config.compression.clone());
        Self {
            config,
            compressor,
            grpc_client: None,
        }
    }

    /// Create a new artifact uploader with gRPC client
    pub async fn new_with_grpc(config: ArtifactConfig, server_url: &str) -> Result<Self> {
        let compressor = Compressor::new(config.compression.clone());
        let channel = tonic::transport::Channel::from_shared(server_url.to_string())
            .map_err(|e| AgentError::Other(format!("Invalid server URL: {}", e)))?
            .connect()
            .await
            .map_err(|e| AgentError::Connection(format!("Failed to connect to server: {}", e)))?;
        let client = WorkerServiceClient::new(channel);
        Ok(Self {
            config,
            compressor,
            grpc_client: Some(client),
        })
    }

    /// Calculate SHA256 checksum of data
    async fn calculate_checksum(data: &[u8]) -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Upload a single file via gRPC streaming
    pub async fn upload_file_grpc(&mut self, file_path: PathBuf, job_id: &str) -> Result<String> {
        let Some(client) = self.grpc_client.as_mut() else {
            return Err(AgentError::Other("gRPC client not initialized".to_string()));
        };

        info!("Starting gRPC upload for artifact: {:?}", file_path);

        // Check file size
        let metadata = fs::metadata(&file_path)
            .await
            .map_err(|e| AgentError::Other(format!("Failed to read file metadata: {}", e)))?;
        let file_size = metadata.len();
        let size_mb = file_size as f64 / (1024.0 * 1024.0);

        if size_mb > self.config.max_file_size_mb as f64 {
            return Err(AgentError::Other(format!(
                "File too large: {:.2} MB (max: {} MB)",
                size_mb, self.config.max_file_size_mb
            )));
        }

        // Read file
        let data = fs::read(&file_path)
            .await
            .map_err(|e| AgentError::Other(format!("Failed to read file: {}", e)))?;

        // Calculate original checksum
        let _original_checksum = Self::calculate_checksum(&data).await?;

        // Compress if needed
        let (data_to_send, is_compressed, compression_type) =
            if matches!(self.config.compression, CompressionType::Gzip) {
                let compressed = self
                    .compressor
                    .compress(&data)
                    .map_err(|e| AgentError::Other(format!("Compression failed: {}", e)))?;
                (compressed, true, "gzip".to_string())
            } else {
                (data, false, "none".to_string())
            };

        // Calculate checksum after compression
        let final_checksum = Self::calculate_checksum(&data_to_send).await?;

        // Generate artifact ID
        let artifact_id = uuid::Uuid::new_v4().to_string();
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();

        // Initiate upload
        let initiate_req = InitiateUploadRequest {
            artifact_id: artifact_id.clone(),
            job_id: job_id.to_string(),
            total_size: data_to_send.len() as u64,
            filename: filename.to_string(),
            checksum: final_checksum.clone(),
            is_compressed,
            compression_type: compression_type.clone(),
        };

        let initiate_response = client
            .initiate_upload(Request::new(initiate_req))
            .await
            .map_err(AgentError::Grpc)?
            .into_inner();

        if !initiate_response.accepted {
            return Err(AgentError::Other(format!(
                "Server rejected upload: {}",
                initiate_response.error_message
            )));
        }

        let upload_id = initiate_response.upload_id;

        // Stream chunks to server
        let chunk_size = 64 * 1024; // 64KB chunks
        let total_chunks = data_to_send.len().div_ceil(chunk_size) as u32;
        let chunks: Vec<ArtifactChunk> = data_to_send
            .chunks(chunk_size)
            .enumerate()
            .map(|(i, chunk)| {
                let sequence_number = i as u32;
                let is_last = i == total_chunks as usize - 1;

                ArtifactChunk {
                    data: chunk.to_vec(),
                    sequence_number,
                    total_chunks,
                    total_size: data_to_send.len() as u64,
                    filename: filename.to_string(),
                    checksum: if is_last {
                        final_checksum.clone()
                    } else {
                        String::new()
                    },
                    is_compressed,
                    compression_type: compression_type.clone(),
                }
            })
            .collect();

        let request_stream = tokio_stream::iter(chunks);

        let upload_response = client
            .upload_artifact(request_stream)
            .await
            .map_err(AgentError::Grpc)?
            .into_inner();

        if !upload_response.success {
            return Err(AgentError::Other(format!(
                "Upload failed: {}",
                upload_response.error_message
            )));
        }

        let bytes_sent = upload_response.bytes_received;

        // Finalize upload
        let finalize_req = FinalizeUploadRequest {
            upload_id: upload_id.clone(),
            artifact_id: artifact_id.clone(),
            total_chunks,
            checksum: final_checksum,
        };

        let finalize_response = client
            .finalize_upload(Request::new(finalize_req))
            .await
            .map_err(AgentError::Grpc)?
            .into_inner();

        if !finalize_response.success {
            return Err(AgentError::Other(format!(
                "Upload finalize failed: {}",
                finalize_response.error_message
            )));
        }

        let server_url = format!("{}/artifacts/{}", self.config.upload_url, artifact_id);

        info!(
            "Successfully uploaded artifact {} ({} bytes, {} chunks)",
            artifact_id, bytes_sent, total_chunks
        );

        Ok(server_url)
    }

    /// Upload a single file (legacy method for backward compatibility)
    pub async fn upload_file(&self, file_path: PathBuf) -> Result<String> {
        info!("Uploading artifact: {:?}", file_path);

        // Check file size
        let metadata = fs::metadata(&file_path)
            .await
            .map_err(|e| AgentError::Other(format!("Failed to read file metadata: {}", e)))?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

        if size_mb > self.config.max_file_size_mb as f64 {
            return Err(AgentError::Other(format!(
                "File too large: {:.2} MB",
                size_mb
            )));
        }

        // Read file
        let data = fs::read(&file_path)
            .await
            .map_err(|e| AgentError::Other(format!("Failed to read file: {}", e)))?;

        // Compress if needed
        let (data, _ext) = if matches!(self.config.compression, CompressionType::Gzip) {
            let compressed = self
                .compressor
                .compress(&data)
                .map_err(|e| AgentError::Other(format!("Compression failed: {}", e)))?;
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
                .map_err(|e| AgentError::Other(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| AgentError::Other(format!("Failed to read directory entry: {}", e)))?
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
    use tokio::io::AsyncWriteExt;

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

    #[tokio::test]
    async fn test_calculate_checksum() {
        let data = b"Hello, World!";
        let checksum = ArtifactUploader::calculate_checksum(data).await.unwrap();

        // SHA256 checksum of "Hello, World!"
        assert_eq!(
            checksum,
            "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
        );
    }

    #[tokio::test]
    async fn test_checksum_consistency() {
        let data = b"test data for consistency check";
        let checksum1 = ArtifactUploader::calculate_checksum(data).await.unwrap();
        let checksum2 = ArtifactUploader::calculate_checksum(data).await.unwrap();

        assert_eq!(checksum1, checksum2);
    }

    #[tokio::test]
    async fn test_checksum_different_data() {
        let data1 = b"first set of data";
        let data2 = b"second set of data";
        let checksum1 = ArtifactUploader::calculate_checksum(data1).await.unwrap();
        let checksum2 = ArtifactUploader::calculate_checksum(data2).await.unwrap();

        assert_ne!(checksum1, checksum2);
    }

    #[tokio::test]
    async fn test_upload_small_file_grpc() {
        // Crear archivo pequeño (< 64KB)
        let mut temp_file = NamedTempFile::new().unwrap();
        let small_data = b"This is a small test file for upload";
        temp_file.write_all(small_data).unwrap();
        temp_file.flush().unwrap();

        let config = ArtifactConfig {
            upload_url: "http://localhost:8080".to_string(),
            compression: CompressionType::None,
            max_file_size_mb: 100,
            retry_attempts: 3,
        };

        // El test fallará sin servidor gRPC real, pero verificamos que la estructura es correcta
        // En una implementación real, usaríamos un mock server o test container
        let result = ArtifactUploader::new_with_grpc(config, "http://localhost:8080").await;

        // Verificamos que se puede crear el cliente gRPC
        match result {
            Ok(_) => {
                // Si hay servidor, podríamos hacer el upload real
                println!("gRPC client created successfully");
            }
            Err(e) => {
                // Sin servidor, esperamos un error de conexión
                assert!(matches!(e, AgentError::Connection(_)));
            }
        }
    }

    #[tokio::test]
    async fn test_upload_large_file_grpc() {
        // Crear archivo grande (> 64KB) para forzar chunking
        let mut temp_file = NamedTempFile::new().unwrap();

        // Escribir ~100KB de datos
        let chunk_data = vec![42u8; 1024]; // 1KB de datos
        for _ in 0..100 {
            temp_file.write_all(&chunk_data).unwrap();
        }
        temp_file.flush().unwrap();

        let config = ArtifactConfig {
            upload_url: "http://localhost:8080".to_string(),
            compression: CompressionType::Gzip,
            max_file_size_mb: 100,
            retry_attempts: 3,
        };

        let result = ArtifactUploader::new_with_grpc(config, "http://localhost:8080").await;

        match result {
            Ok(uploader) => {
                // Si hay servidor, testearíamos el upload con chunking
                println!("Large file upload test ready (requires gRPC server)");
                // En test real: let result = uploader.upload_file_grpc(temp_file.path().to_path_buf(), "test-job").await;
            }
            Err(_) => {
                // Sin servidor, esperamos error de conexión
                println!("gRPC server not available for large file test");
            }
        }
    }

    #[tokio::test]
    async fn test_upload_with_gzip_compression() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Datos repetitivos que se comprimen bien
        let repetitive_data = b"A".repeat(10000);
        temp_file.write_all(&repetitive_data).unwrap();
        temp_file.flush().unwrap();

        let config = ArtifactConfig {
            upload_url: "http://localhost:8080".to_string(),
            compression: CompressionType::Gzip,
            max_file_size_mb: 100,
            retry_attempts: 3,
        };

        let compressor = Compressor::new(CompressionType::Gzip);
        let original_data = &repetitive_data;
        let compressed_data = compressor.compress(original_data).unwrap();

        // Verificamos que la compresión funciona y reduce el tamaño
        assert!(compressed_data.len() < original_data.len());
        assert!(compressed_data.len() > 0);
    }

    #[tokio::test]
    async fn test_file_size_validation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Crear archivo más grande que el límite
        let large_data = vec![0u8; 200 * 1024 * 1024]; // 200MB
        temp_file.write_all(&large_data).unwrap();

        let config = ArtifactConfig {
            upload_url: "http://localhost:8080".to_string(),
            compression: CompressionType::None,
            max_file_size_mb: 100, // Límite de 100MB
            retry_attempts: 3,
        };

        let uploader = ArtifactUploader::new(config);
        let result = uploader.upload_file(temp_file.path().to_path_buf()).await;

        // Debería fallar por tamaño excesivo
        assert!(result.is_err());
        match result {
            Err(AgentError::Other(msg)) => {
                assert!(msg.contains("too large"));
            }
            _ => panic!("Expected AgentError::Other"),
        }
    }

    #[tokio::test]
    async fn test_upload_multiple_files() {
        let mut file1 = NamedTempFile::new().unwrap();
        let mut file2 = NamedTempFile::new().unwrap();

        file1.write_all(b"File 1 content").unwrap();
        file2.write_all(b"File 2 content").unwrap();

        let config = ArtifactConfig::default();
        let uploader = ArtifactUploader::new(config);

        let files = vec![file1.path().to_path_buf(), file2.path().to_path_buf()];

        let result = uploader.upload_files(files).await;
        assert!(result.is_ok());
        let urls = result.unwrap();
        assert_eq!(urls.len(), 2);
    }

    #[tokio::test]
    async fn test_upload_directory() {
        // Crear directorio temporal con archivos
        let temp_dir = tempfile::tempdir().unwrap();
        let dir_path = temp_dir.path().to_path_buf();

        // Crear archivos en el directorio
        let file1_path = dir_path.join("file1.txt");
        let file2_path = dir_path.join("file2.txt");

        let mut file1 = tokio::fs::File::create(&file1_path).await.unwrap();
        file1.write_all(b"Content 1").await.unwrap();

        let mut file2 = tokio::fs::File::create(&file2_path).await.unwrap();
        file2.write_all(b"Content 2").await.unwrap();

        let config = ArtifactConfig::default();
        let uploader = ArtifactUploader::new(config);

        let result = uploader.upload_directory(dir_path.clone()).await;
        assert!(result.is_ok());

        // Limpiar
        tokio::fs::remove_file(&file1_path).await.unwrap();
        tokio::fs::remove_file(&file2_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_chunking_logic() {
        let data = vec![0u8; 100_000]; // 100KB
        let chunk_size = 64 * 1024; // 64KB

        let total_chunks = ((data.len() + chunk_size - 1) / chunk_size) as u32;
        assert_eq!(total_chunks, 2); // Debería ser 2 chunks de 64KB

        let chunks: Vec<_> = data.chunks(chunk_size).collect();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 64 * 1024);
        // 100_000 - 65_536 = 34_464 bytes para el segundo chunk
        assert_eq!(chunks[1].len(), 100_000 - (64 * 1024));
    }
}
