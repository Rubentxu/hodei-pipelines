//! Logging module
//!
//! This module implements intelligent log buffering and streaming for job output.
//! It buffers logs in chunks and flushes based on size or time.

pub mod buffer;
pub mod streaming;

pub use buffer::{BufferConfig, LogBuffer, LogChunk};
pub use streaming::{LogStreamer, StreamConfig};
