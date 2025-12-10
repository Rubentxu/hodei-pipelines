//! Compression module
//!
//! This module provides compression utilities for artifact uploads.

use flate2::{Compression, write::GzEncoder};
use std::io::{self, Write};
use thiserror::Error;

/// Compression types supported
#[derive(Debug, Clone)]
#[derive(Default)]
pub enum CompressionType {
    None,
    #[default]
    Gzip,
}


/// Compression error
#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Compression failed: {0}")]
    Compression(String),
}

/// Compressor for artifact files
#[derive(Debug)]
pub struct Compressor {
    compression_type: CompressionType,
}

impl Compressor {
    /// Create a new compressor
    pub fn new(compression_type: CompressionType) -> Self {
        Self { compression_type }
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                let compressed = encoder
                    .finish()
                    .map_err(|e| CompressionError::Compression(e.to_string()))?;
                Ok(compressed)
            }
        }
    }

    /// Compress a file
    pub fn compress_file<P: AsRef<std::path::Path>>(
        &self,
        input_path: P,
        output_path: P,
    ) -> Result<(), CompressionError> {
        let input = std::fs::File::open(input_path)?;
        let mut input_reader = io::BufReader::new(input);

        let output = std::fs::File::create(output_path)?;
        let mut output_writer = io::BufWriter::new(output);

        match self.compression_type {
            CompressionType::None => {
                io::copy(&mut input_reader, &mut output_writer)?;
            }
            CompressionType::Gzip => {
                let mut encoder = GzEncoder::new(&mut output_writer, Compression::default());
                io::copy(&mut input_reader, &mut encoder)?;
                encoder.finish()?;
            }
        }

        output_writer.flush()?;
        Ok(())
    }

    /// Get compression type
    pub fn compression_type(&self) -> &CompressionType {
        &self.compression_type
    }

    /// Get file extension for compression type
    pub fn extension(&self) -> &'static str {
        match self.compression_type {
            CompressionType::None => "",
            CompressionType::Gzip => "gz",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_creation() {
        let compressor = Compressor::new(CompressionType::default());
        assert!(matches!(
            compressor.compression_type(),
            CompressionType::Gzip
        ));
    }

    #[test]
    fn test_compress_data() {
        let compressor = Compressor::new(CompressionType::Gzip);
        let data = b"Hello, World!";
        let compressed = compressor.compress(data).unwrap();

        // Compressed should be different from original
        assert!(compressed.len() > 0);
    }
}
