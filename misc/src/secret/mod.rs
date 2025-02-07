pub mod aes;
pub mod config;
pub mod factory;

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as B64Engine;
use base64::Engine;

/// Trait for data encryption and decryption operations.
///
/// Implementors of this trait provide methods to encrypt and decrypt data,
/// ensuring secure data transmission over networks.
pub trait Secret {
    /// Encrypts the provided data.
    ///
    /// # Arguments
    /// * `data` - Raw bytes to encrypt
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Encrypted data as bytes
    /// * `Err` - If encryption fails
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Decrypts the provided data.
    ///
    /// # Arguments
    /// * `data` - Encrypted bytes to decrypt
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Decrypted data as bytes
    /// * `Err` - If decryption fails
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
}

/// Encodes binary data to base64 string.
///
/// # Arguments
/// * `data` - Raw bytes to encode
///
/// # Returns
/// Base64 encoded string
///
/// # Examples
/// ```
/// use csync_misc::secret::base64_encode;
///
/// let data = b"Hello, World!";
/// let encoded = base64_encode(data);
/// assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn base64_encode(data: &[u8]) -> String {
    B64Engine.encode(data)
}

/// Decodes base64 string to binary data.
///
/// # Arguments
/// * `data` - Base64 encoded string
///
/// # Returns
/// * `Ok(Vec<u8>)` - Decoded bytes
/// * `Err` - If input is not valid base64
///
/// # Examples
/// ```
/// use csync_misc::secret::base64_decode;
///
/// let encoded = "SGVsbG8sIFdvcmxkIQ==";
/// let decoded = base64_decode(encoded).unwrap();
/// assert_eq!(decoded, b"Hello, World!");
/// ```
pub fn base64_decode(data: &str) -> Result<Vec<u8>> {
    B64Engine.decode(data).context("invalid base64 content")
}
