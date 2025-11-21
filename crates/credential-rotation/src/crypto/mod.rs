//! Cryptographic Operations Module
//!
//! This module provides cryptographic utilities for secure credential storage and rotation.

use crate::CredentialRotationError;
use rand::Rng;

/// Generate a random key
pub fn generate_random_key(length: usize) -> Result<Vec<u8>, CredentialRotationError> {
    let mut rng = rand::thread_rng();
    let key: Vec<u8> = (0..length).map(|_| rng.gen::<u8>()).collect();
    Ok(key)
}

/// Derive a key from a password using a simplified approach
/// NOTE: This is simplified for demo purposes. Use PBKDF2/scrypt/argon2 in production
pub fn derive_key_from_password(
    password: &[u8],
    salt: &[u8],
    _iterations: u32,
    key_length: usize,
) -> Result<Vec<u8>, CredentialRotationError> {
    // Simplified key derivation for demo
    // In production, use proper algorithms like PBKDF2, scrypt, or argon2
    let mut key = vec![0u8; key_length];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = password.get(i % password.len()).copied().unwrap_or(0)
            ^ salt.get(i % salt.len()).copied().unwrap_or(0)
            ^ (i as u8);
    }
    Ok(key)
}

/// Encrypt data using a simple XOR operation
/// NOTE: This is NOT secure for production use. Use AES-GCM or similar.
pub fn simple_encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, CredentialRotationError> {
    if key.len() < 32 {
        return Err(CredentialRotationError::CryptoError(
            "Key must be at least 32 bytes for AES-256".to_string(),
        ));
    }

    // XOR with key (NOT secure for production - use proper encryption!)
    let mut result = Vec::with_capacity(data.len());
    for (i, byte) in data.iter().enumerate() {
        result.push(byte ^ key[i % key.len()]);
    }
    Ok(result)
}

/// Decrypt data
pub fn simple_decrypt(
    encrypted_data: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, CredentialRotationError> {
    // XOR is symmetric, so we can use the same operation
    simple_encrypt(encrypted_data, key)
}
