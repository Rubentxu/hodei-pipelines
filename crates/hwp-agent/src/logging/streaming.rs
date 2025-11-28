//! Log streaming module
//!
//! This module handles streaming logs from PTY to gRPC with buffering
//! and backpressure handling.

use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use super::buffer::{LogBuffer, LogChunk, StreamType};
use crate::{AgentError, Result};

/// Stream configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub buffer_size: usize,
    pub flush_interval_ms: u64,
    pub backpressure_threshold: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 4096,
            flush_interval_ms: 100,
            backpressure_threshold: 8192,
        }
    }
}

/// Log streamer for streaming PTY output
#[derive(Debug)]
pub struct LogStreamer {
    job_id: String,
    buffer: LogBuffer,
    stream_type: StreamType,
    grpc_sender: Option<mpsc::UnboundedSender<LogChunk>>,
}

impl LogStreamer {
    /// Create a new log streamer
    pub fn new(job_id: String, stream_type: StreamType, buffer: LogBuffer) -> Self {
        Self {
            job_id,
            buffer,
            stream_type,
            grpc_sender: None,
        }
    }

    /// Set gRPC sender
    pub fn set_sender(&mut self, sender: mpsc::UnboundedSender<LogChunk>) {
        self.grpc_sender = Some(sender);
    }

    /// Stream from a reader (PTY master)
    pub async fn stream_from_reader<R>(&mut self, reader: &mut R) -> Result<()>
    where
        R: AsyncReadExt + Send + Sync + Unpin,
    {
        let mut buf = vec![0u8; 8192];

        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    debug!("End of stream for job {}", self.job_id);
                    break;
                }
                Ok(n) => {
                    let data = buf[..n].to_vec();

                    // Add to buffer
                    self.buffer
                        .add_chunk(self.job_id.clone(), self.stream_type.clone(), data)
                        .await;

                    // Flush if buffer is getting full
                    if self.buffer.size() > 10 {
                        self.flush().await?;
                    }
                }
                Err(e) => {
                    error!("Error reading from stream: {}", e);
                    return Err(e.into());
                }
            }
        }

        // Flush remaining data
        self.flush().await?;

        Ok(())
    }

    /// Flush the buffer
    pub async fn flush(&mut self) -> Result<()> {
        let chunks = self.buffer.get_chunks_for_flush();

        if let Some(sender) = &self.grpc_sender {
            for chunk in chunks {
                if sender.send(chunk).is_err() {
                    warn!("Failed to send log chunk to gRPC (receiver closed)");
                    return Err(AgentError::Connection("gRPC sender closed".to_string()).into());
                }
            }
        }

        Ok(())
    }

    /// Get job ID
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
}

/// Async log reader wrapper
#[derive(Debug)]
pub struct LogReader {
    streamer: LogStreamer,
}

impl LogReader {
    /// Create a new log reader
    pub fn new(job_id: String, stream_type: StreamType, buffer: LogBuffer) -> Self {
        Self {
            streamer: LogStreamer::new(job_id, stream_type, buffer),
        }
    }

    /// Set gRPC sender
    pub fn set_sender(&mut self, sender: mpsc::UnboundedSender<LogChunk>) {
        self.streamer.set_sender(sender);
    }

    /// Start streaming from PTY
    pub async fn start_streaming<R>(&mut self, reader: &mut R) -> Result<()>
    where
        R: AsyncReadExt + Send + Sync + Unpin,
    {
        self.streamer.stream_from_reader(reader).await
    }

    /// Flush buffered logs
    pub async fn flush(&mut self) -> Result<()> {
        self.streamer.flush().await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamingError {
    #[error("Read error: {0}")]
    Read(String),

    #[error("Write error: {0}")]
    Write(String),

    #[error("Buffer overflow")]
    BufferOverflow,
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[tokio::test]
    async fn test_log_streamer_creation() {
        let buffer = LogBuffer::new(Default::default());
        let streamer = LogStreamer::new("job-1".to_string(), StreamType::Stdout, buffer);
        assert_eq!(streamer.job_id(), "job-1");
    }
}
