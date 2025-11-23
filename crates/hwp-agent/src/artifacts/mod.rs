//! Artifact management module
//!
//! This module handles uploading job artifacts to the server or storage.

pub mod compression;
pub mod uploader;

pub use compression::{CompressionType, Compressor};
pub use uploader::{ArtifactConfig, ArtifactUploader};
