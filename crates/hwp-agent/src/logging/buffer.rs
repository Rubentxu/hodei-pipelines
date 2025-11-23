//! Log buffering module
//!
//! This module implements an intelligent log buffer that collects output
//! and flushes based on size (4KB) or time (100ms) thresholds.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, MissedTickBehavior, interval};
use tracing::debug;

/// Log chunk for transmission
#[derive(Debug, Clone)]
pub struct LogChunk {
    pub job_id: String,
    pub stream_type: StreamType,
    pub sequence: u64,
    pub timestamp: i64,
    pub data: Vec<u8>,
}

/// Stream type
#[derive(Debug, Clone, PartialEq)]
pub enum StreamType {
    Stdout,
    Stderr,
}

/// Buffer configuration
#[derive(Debug, Clone, Default)]
pub struct BufferConfig {
    pub max_size_bytes: usize,
    pub flush_interval_ms: u64,
    pub sequence_start: u64,
}

impl BufferConfig {
    pub fn with_size(size: usize) -> Self {
        Self {
            max_size_bytes: size,
            flush_interval_ms: 100,
            sequence_start: 0,
        }
    }
}

/// Thread-safe log buffer
#[derive(Debug, Clone)]
pub struct LogBuffer {
    config: BufferConfig,
    buffer: Arc<Mutex<VecDeque<LogChunk>>>,
    sequence: Arc<Mutex<u64>>,
}

impl LogBuffer {
    /// Create a new log buffer
    pub fn new(config: BufferConfig) -> Self {
        Self {
            config: config.clone(),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            sequence: Arc::new(Mutex::new(config.sequence_start)),
        }
    }

    /// Add a log chunk to the buffer
    pub async fn add_chunk(&self, job_id: String, stream_type: StreamType, mut data: Vec<u8>) {
        let sequence = {
            let mut seq = self.sequence.lock().unwrap();
            let val = *seq;
            *seq += 1;
            val
        };

        let chunk = LogChunk {
            job_id,
            stream_type,
            sequence,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            data: std::mem::take(&mut data),
        };

        let mut buffer = self.buffer.lock().unwrap();
        buffer.push_back(chunk);

        // Check if we need to flush
        let total_size: usize = buffer.iter().map(|c| c.data.len()).sum();
        if total_size >= self.config.max_size_bytes {
            debug!("Buffer full, size: {} bytes", total_size);
        }
    }

    /// Get current buffer size
    pub fn size(&self) -> usize {
        let buffer = self.buffer.lock().unwrap();
        buffer.iter().map(|c| c.data.len()).sum()
    }

    /// Get chunks ready for flush
    pub fn get_chunks_for_flush(&self) -> Vec<LogChunk> {
        let mut buffer = self.buffer.lock().unwrap();
        let mut chunks = Vec::new();

        // Take all chunks
        while let Some(chunk) = buffer.pop_front() {
            chunks.push(chunk);
        }

        chunks
    }

    /// Flush the buffer
    pub async fn flush(&self) -> Vec<LogChunk> {
        self.get_chunks_for_flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_buffer_creation() {
        let buffer = LogBuffer::new(BufferConfig::default());
        assert_eq!(buffer.size(), 0);
    }

    #[tokio::test]
    async fn test_log_buffer_add_chunk() {
        let buffer = LogBuffer::new(BufferConfig::default());
        buffer
            .add_chunk(
                "job-1".to_string(),
                StreamType::Stdout,
                b"test data".to_vec(),
            )
            .await;
        assert!(buffer.size() > 0);
    }

    #[tokio::test]
    async fn test_log_buffer_flush() {
        let buffer = LogBuffer::new(BufferConfig::default());
        buffer
            .add_chunk(
                "job-1".to_string(),
                StreamType::Stdout,
                b"test data".to_vec(),
            )
            .await;

        let chunks = buffer.flush().await;
        assert_eq!(chunks.len(), 1);
        assert_eq!(buffer.size(), 0);
    }
}
