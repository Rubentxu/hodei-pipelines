//! Artifact management module
//!
//! This module handles uploading job artifacts to the server or storage.

pub mod compression;
pub mod text_replacer;
pub mod uploader;

pub use compression::{CompressionType, Compressor};
pub use text_replacer::{AhoCorasickReplacer, ReplacementPattern, ReplacerError};
pub use uploader::{ArtifactConfig, ArtifactUploader};
