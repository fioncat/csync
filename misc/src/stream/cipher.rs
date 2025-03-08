use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, Nonce, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use anyhow::{bail, Result};
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;

use crate::code;

/// A cipher implementation for encrypting and decrypting data using AES-256-GCM.
///
/// This struct provides methods for secure encryption and decryption of data
/// using the AES-256-GCM algorithm with PBKDF2 key derivation.
#[derive(Debug, Clone)]
pub struct Cipher {
    /// The encryption key
    key: Vec<u8>,
}

impl Cipher {
    /// Length of the salt used for key derivation
    const SALT_LENGTH: usize = 30;
    /// Length of the nonce used for encryption
    const NONCE_LENGTH: usize = 12;
    /// Combined length of salt and nonce in the encrypted data header
    const HEAD_LENGTH: usize = Self::SALT_LENGTH + Self::NONCE_LENGTH;

    /// Number of rounds used for PBKDF2 key derivation
    const PBKDF2_ROUNDS: u32 = 600;

    /// Length of the generated random key
    const GENERATE_KEY_LENGTH: usize = 100;

    /// Creates a new Cipher instance with the provided key.
    ///
    /// # Arguments
    ///
    /// * `key` - The encryption key to use
    ///
    /// # Returns
    ///
    /// A new Cipher instance
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Generates a random key suitable for encryption.
    ///
    /// # Returns
    ///
    /// A randomly generated key as a Vec<u8>
    pub fn generate_key() -> Vec<u8> {
        let mut key = vec![0u8; Self::GENERATE_KEY_LENGTH];
        OsRng.fill_bytes(&mut key);
        let key = code::base64_encode(&key);
        key.into_bytes()
    }

    /// Extracts the salt and nonce from the encrypted data header.
    ///
    /// # Arguments
    ///
    /// * `data` - The encrypted data containing the salt and nonce in its header
    ///
    /// # Returns
    ///
    /// A tuple containing the salt and nonce as fixed-size arrays
    fn get_salt_nonce(&self, data: &[u8]) -> ([u8; Self::SALT_LENGTH], [u8; Self::NONCE_LENGTH]) {
        let mut salt = [0u8; Self::SALT_LENGTH];
        let mut nonce = [0u8; Self::NONCE_LENGTH];
        salt.copy_from_slice(&data[..Self::SALT_LENGTH]);
        nonce.copy_from_slice(&data[Self::SALT_LENGTH..Self::HEAD_LENGTH]);
        (salt, nonce)
    }

    /// Encrypts data using AES-256-GCM.
    ///
    /// The encrypted data format includes:
    /// - Salt (30 bytes) for key derivation
    /// - Nonce (12 bytes) for encryption
    /// - Encrypted data
    ///
    /// # Arguments
    ///
    /// * `data` - The data to encrypt
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The encrypted data
    /// * `Err` - If encryption fails
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        let mut rng = OsRng;
        let mut salt = [0; Self::SALT_LENGTH];
        rng.fill_bytes(&mut salt);

        let key = pbkdf2_hmac_array::<Sha256, 32>(&self.key, &salt, Self::PBKDF2_ROUNDS);
        let key = Key::<Aes256Gcm>::from_slice(&key);

        let cipher = Aes256Gcm::new(key);
        // Generate the nonce in aes-256-gcm.
        let nonce = Aes256Gcm::generate_nonce(&mut rng);
        assert_eq!(nonce.len(), Self::NONCE_LENGTH);

        let mut ret = salt.to_vec();
        ret.extend(nonce.to_vec());

        let encrypted = match cipher.encrypt(&nonce, data) {
            Ok(data) => data,
            Err(err) => bail!("use aes256gcm to encrypt data: {err}"),
        };
        ret.extend(encrypted);

        Ok(ret)
    }

    /// Decrypts data that was encrypted using the encrypt method.
    ///
    /// # Arguments
    ///
    /// * `data` - The encrypted data to decrypt
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The decrypted data
    /// * `Err` - If decryption fails
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }

        if data.len() < Self::HEAD_LENGTH {
            bail!("data missing salt and nonce");
        }

        let (salt, nonce) = self.get_salt_nonce(data);
        let data = &data[Self::HEAD_LENGTH..];
        if data.is_empty() {
            bail!("empty data to decrypt");
        }

        let key = pbkdf2_hmac_array::<Sha256, 32>(&self.key, &salt, Self::PBKDF2_ROUNDS);
        let key = Key::<Aes256Gcm>::from_slice(&key);
        let cipher = Aes256Gcm::new(key);

        let nonce = Nonce::<Aes256Gcm>::from_slice(&nonce);

        let ret = match cipher.decrypt(nonce, data) {
            Ok(data) => data,
            Err(err) => bail!("use aes256gcm to decrypt data: {err}"),
        };

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key1 = Cipher::generate_key();
        let key2 = Cipher::generate_key();

        // Keys should be non-empty
        assert!(!key1.is_empty());
        assert!(!key2.is_empty());

        // Keys should be different (random)
        assert_ne!(key1, key2);

        // Keys should have the expected length after base64 encoding
        assert!(key1.len() > Cipher::GENERATE_KEY_LENGTH);
    }

    #[test]
    fn test_encrypt_decrypt_empty() {
        let cipher = Cipher::new(b"test_key".to_vec());

        // Test with empty data
        let empty_data = b"";
        let encrypted = cipher.encrypt(empty_data).unwrap();
        assert!(encrypted.is_empty());

        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, empty_data);
    }

    #[test]
    fn test_encrypt_decrypt_small_data() {
        let cipher = Cipher::new(b"test_key".to_vec());

        // Test with small data
        let data = b"Hello, world!";
        let encrypted = cipher.encrypt(data).unwrap();

        // Encrypted data should be longer than original due to salt and nonce
        assert!(encrypted.len() > data.len());
        assert!(encrypted.len() >= Cipher::HEAD_LENGTH);

        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let cipher = Cipher::new(b"test_key".to_vec());

        // Test with larger data
        let data = vec![0x42; 1024]; // 1KB of data
        let encrypted = cipher.encrypt(&data).unwrap();

        // Encrypted data should be longer than original due to salt and nonce
        assert!(encrypted.len() > data.len());

        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_different_keys() {
        let data = b"Secret message";

        // Encrypt with one key
        let cipher1 = Cipher::new(b"key1".to_vec());
        let encrypted = cipher1.encrypt(data).unwrap();

        // Try to decrypt with a different key
        let cipher2 = Cipher::new(b"key2".to_vec());
        let result = cipher2.decrypt(&encrypted);

        // Decryption should fail with a different key
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_data() {
        let cipher = Cipher::new(b"test_key".to_vec());
        let data = b"Important data";

        // Encrypt the data
        let mut encrypted = cipher.encrypt(data).unwrap();

        // Tamper with the encrypted data (if it's long enough)
        if encrypted.len() > Cipher::HEAD_LENGTH + 1 {
            encrypted[Cipher::HEAD_LENGTH + 1] ^= 0xFF; // Flip bits in the first byte of actual encrypted data

            // Decryption should fail
            let result = cipher.decrypt(&encrypted);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_invalid_data_length() {
        let cipher = Cipher::new(b"test_key".to_vec());

        // Data shorter than HEAD_LENGTH
        let short_data = vec![0; Cipher::HEAD_LENGTH - 1];
        let result = cipher.decrypt(&short_data);
        assert!(result.is_err());

        // Data exactly HEAD_LENGTH but no actual encrypted content
        let exact_head_data = vec![0; Cipher::HEAD_LENGTH];
        let result = cipher.decrypt(&exact_head_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip_with_different_data_types() {
        let cipher = Cipher::new(b"test_key".to_vec());

        // Test with string data
        let string_data = "Hello, world!".as_bytes();
        let encrypted = cipher.encrypt(string_data).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, string_data);

        // Test with binary data
        let binary_data = vec![0x00, 0xFF, 0x42, 0x13, 0x37];
        let encrypted = cipher.encrypt(&binary_data).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, binary_data);
    }
}
