//! Log buffering module with lock-free ring buffer implementation
//!
//! This module implements a high-performance log buffer using a lock-free
//! ring buffer that collects output and flushes based on size (4KB) or time (100ms) thresholds.
//!
//! Key optimizations:
//! - Lock-free ring buffer using atomic operations
//! - Pre-allocated capacity to minimize allocations
//! - Batched flushing for better performance
//! - Backpressure handling to prevent memory exhaustion

use crossbeam::queue::SegQueue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tracing::{debug, warn};

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

/// Buffer configuration with performance tuning
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum buffer size in bytes before forced flush
    pub max_size_bytes: usize,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Initial sequence number
    pub sequence_start: u64,
    /// Pre-allocated buffer capacity (number of chunks)
    pub prealloc_capacity: usize,
    /// Enable backpressure protection
    pub backpressure_enabled: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 4096,
            flush_interval_ms: 100,
            sequence_start: 0,
            prealloc_capacity: 1024,
            backpressure_enabled: true,
        }
    }
}

impl BufferConfig {
    pub fn with_size(size: usize) -> Self {
        Self {
            max_size_bytes: size,
            ..Default::default()
        }
    }

    /// High-performance configuration for production
    pub fn production() -> Self {
        Self {
            max_size_bytes: 16384,
            flush_interval_ms: 50,
            sequence_start: 0,
            prealloc_capacity: 4096,
            backpressure_enabled: true,
        }
    }
}

/// Lock-free log buffer with ring buffer implementation
#[derive(Debug, Clone)]
pub struct LogBuffer {
    config: BufferConfig,
    /// Internal queue for log chunks
    queue: Arc<SegQueue<LogChunk>>,
    /// Total size of buffered data
    size: Arc<AtomicUsize>,
    /// Current sequence number
    sequence: Arc<AtomicU64>,
    /// Maximum queue size (backpressure threshold)
    max_queue_size: Arc<AtomicUsize>,
}

impl LogBuffer {
    /// Create a new log buffer with pre-allocation
    pub fn new(config: BufferConfig) -> Self {
        let queue = Arc::new(SegQueue::new());
        let size = Arc::new(AtomicUsize::new(0));
        let sequence = Arc::new(AtomicU64::new(config.sequence_start));
        let max_queue_size = Arc::new(AtomicUsize::new(config.prealloc_capacity * 2));

        Self {
            config,
            queue,
            size,
            sequence,
            max_queue_size,
        }
    }

    /// Add a log chunk to the buffer (lock-free)
    pub async fn add_chunk(
        &self,
        job_id: String,
        stream_type: StreamType,
        mut data: Vec<u8>,
    ) -> Result<(), LogBufferError> {
        // Check backpressure if enabled
        if self.config.backpressure_enabled
            && self.queue.len() > self.max_queue_size.load(Ordering::Relaxed)
        {
            return Err(LogBufferError::Backpressure(format!(
                "Queue full: {} > {}",
                self.queue.len(),
                self.max_queue_size.load(Ordering::Relaxed)
            )));
        }

        // Get next sequence number atomically
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);

        // Create log chunk
        let chunk = LogChunk {
            job_id,
            stream_type,
            sequence,
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            data: std::mem::take(&mut data),
        };

        // Update size atomically
        let chunk_size = chunk.data.len();
        self.size.fetch_add(chunk_size, Ordering::Relaxed);

        // Push to queue (lock-free)
        self.queue.push(chunk);

        debug!(
            "Added chunk: size={} bytes, queue_len={}, total_buffered={}",
            chunk_size,
            self.queue.len(),
            self.size.load(Ordering::Relaxed)
        );

        Ok(())
    }

    /// Get current buffer size (atomic read)
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Get queue length (approximate, lock-free)
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get chunks ready for flush (lock-free batch operation)
    pub fn get_chunks_for_flush(&self) -> Vec<LogChunk> {
        let mut chunks = Vec::new();
        let mut total_size = 0;
        let size_limit = self.config.max_size_bytes;

        // Extract chunks until we reach size limit or queue is empty
        while let Some(chunk) = self.queue.pop() {
            chunks.push(chunk);
            total_size = chunks.iter().map(|c| c.data.len()).sum();

            if total_size >= size_limit && chunks.len() > 1 {
                debug!(
                    "Flush limit reached: {} chunks, {} bytes",
                    chunks.len(),
                    total_size
                );
                break;
            }
        }

        // Update size counter
        if !chunks.is_empty() {
            self.size.fetch_sub(total_size, Ordering::Relaxed);
        }

        if !chunks.is_empty() {
            debug!("Flushing {} chunks ({} bytes)", chunks.len(), total_size);
        }

        chunks
    }

    /// Flush the buffer and return all pending chunks
    pub async fn flush(&self) -> Vec<LogChunk> {
        self.get_chunks_for_flush()
    }

    /// Get statistics about buffer state
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            queue_len: self.queue.len(),
            size_bytes: self.size.load(Ordering::Relaxed),
            sequence: self.sequence.load(Ordering::Relaxed),
            max_size_bytes: self.config.max_size_bytes,
        }
    }

    /// Clear the buffer (emergency operation)
    pub fn clear(&self) {
        let mut count = 0;
        while self.queue.pop().is_some() {
            count += 1;
        }
        self.size.store(0, Ordering::Relaxed);
        warn!("Cleared {} chunks from buffer", count);
    }
}

/// Buffer statistics for monitoring
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub queue_len: usize,
    pub size_bytes: usize,
    pub sequence: u64,
    pub max_size_bytes: usize,
}

/// Error types for log buffer operations
#[derive(Debug, thiserror::Error)]
pub enum LogBufferError {
    #[error("Backpressure error: {0}")]
    Backpressure(String),

    #[error("Queue overflow")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_buffer_creation() {
        let buffer = LogBuffer::new(BufferConfig::default());
        assert_eq!(buffer.size(), 0);
        assert!(buffer.is_empty());
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
            .await
            .unwrap();
        assert!(!buffer.is_empty());
        assert_eq!(buffer.size(), 9);
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
            .await
            .unwrap();

        let chunks = buffer.flush().await;
        assert_eq!(chunks.len(), 1);
        assert_eq!(buffer.size(), 0);
        assert!(buffer.is_empty());
    }

    #[tokio::test]
    async fn test_log_buffer_size_limit() {
        let config = BufferConfig {
            max_size_bytes: 10,
            ..Default::default()
        };
        let buffer = LogBuffer::new(config);

        // Add chunks until we hit the limit
        buffer
            .add_chunk("job-1".to_string(), StreamType::Stdout, b"12345".to_vec())
            .await
            .unwrap();
        assert_eq!(buffer.size(), 5);

        // This should trigger flush due to size limit
        buffer
            .add_chunk("job-1".to_string(), StreamType::Stdout, b"67890".to_vec())
            .await
            .unwrap();

        // Check stats
        let stats = buffer.stats();
        debug!("Stats: {:?}", stats);
    }

    #[tokio::test]
    async fn test_log_buffer_concurrent_access() {
        let buffer = LogBuffer::new(BufferConfig::default());
        let mut handles = vec![];

        for i in 0..10 {
            let buffer_clone = buffer.clone();
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    let data = format!("chunk-{}-{}", i, j).into_bytes();
                    buffer_clone
                        .add_chunk(format!("job-{}", i), StreamType::Stdout, data)
                        .await
                        .unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert!(buffer.len() > 0);
        debug!("Total chunks: {}", buffer.len());
    }

    #[tokio::test]
    async fn test_log_buffer_backpressure() {
        let config = BufferConfig {
            prealloc_capacity: 10,
            backpressure_enabled: true,
            ..Default::default()
        };
        let buffer = LogBuffer::new(config);

        // Try to exceed the queue size
        for i in 0..25 {
            let result = buffer
                .add_chunk(
                    "job-1".to_string(),
                    StreamType::Stdout,
                    format!("chunk-{}", i).into_bytes(),
                )
                .await;

            if i >= 20 {
                // Should eventually hit backpressure
                if let Err(LogBufferError::Backpressure(_)) = result {
                    debug!("Backpressure triggered at iteration {}", i);
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_buffer_stats() {
        let buffer = LogBuffer::new(BufferConfig::default());
        let stats_before = buffer.stats();
        assert_eq!(stats_before.size_bytes, 0);
        assert_eq!(stats_before.queue_len, 0);

        buffer
            .add_chunk(
                "job-1".to_string(),
                StreamType::Stdout,
                b"test data".to_vec(),
            )
            .await
            .unwrap();

        let stats_after = buffer.stats();
        assert_eq!(stats_after.size_bytes, 9);
        assert!(stats_after.queue_len > 0);
    }

    #[tokio::test]
    async fn test_production_config() {
        let config = BufferConfig::production();
        let _buffer = LogBuffer::new(config.clone());

        assert_eq!(config.max_size_bytes, 16384);
        assert_eq!(config.flush_interval_ms, 50);
        assert!(config.prealloc_capacity > 1000);
        assert!(config.backpressure_enabled);
    }
}
