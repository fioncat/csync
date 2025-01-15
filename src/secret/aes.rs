use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, Nonce, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use anyhow::{bail, Result};
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;

use super::{base64_encode, Secret};

#[derive(Debug, Clone)]
pub struct AesSecret {
    key: Vec<u8>,
}

impl AesSecret {
    const SALT_LENGTH: usize = 30;
    const NONCE_LENGTH: usize = 12;
    const HEAD_LENGTH: usize = Self::SALT_LENGTH + Self::NONCE_LENGTH;

    const PBKDF2_ROUNDS: u32 = 600;

    const GENERATE_KEY_LENGTH: usize = 100;

    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    pub fn generate_key() -> Vec<u8> {
        let mut key = vec![0u8; Self::GENERATE_KEY_LENGTH];
        OsRng.fill_bytes(&mut key);
        let key = base64_encode(&key);
        key.into_bytes()
    }

    fn get_salt_nonce(&self, data: &[u8]) -> ([u8; Self::SALT_LENGTH], [u8; Self::NONCE_LENGTH]) {
        let mut salt = [0u8; Self::SALT_LENGTH];
        let mut nonce = [0u8; Self::NONCE_LENGTH];
        salt.copy_from_slice(&data[..Self::SALT_LENGTH]);
        nonce.copy_from_slice(&data[Self::SALT_LENGTH..Self::HEAD_LENGTH]);
        (salt, nonce)
    }
}

impl Secret for AesSecret {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
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

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
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
    fn test_aes_encrypt_decrypt() {
        // Generate a key
        let key = AesSecret::generate_key();
        let secret = AesSecret { key };

        // Test case 1: Regular string
        let original_data = b"Hello, World!";
        let encrypted = secret.encrypt(original_data).unwrap();
        let decrypted = secret.decrypt(&encrypted).unwrap();
        assert_eq!(original_data, decrypted.as_slice());

        // Test case 2: Empty data
        let empty_data = b"";
        let encrypted = secret.encrypt(empty_data).unwrap();
        let decrypted = secret.decrypt(&encrypted).unwrap();
        assert_eq!(empty_data, decrypted.as_slice());

        // Test case 3: Binary data
        let binary_data = vec![1, 2, 3, 4, 5, 255, 254, 253];
        let encrypted = secret.encrypt(&binary_data).unwrap();
        let decrypted = secret.decrypt(&encrypted).unwrap();
        assert_eq!(binary_data, decrypted);

        // Test case 4: Large amount of data
        let large_data = vec![42u8; 1000];
        let encrypted = secret.encrypt(&large_data).unwrap();
        let decrypted = secret.decrypt(&encrypted).unwrap();
        assert_eq!(large_data, decrypted);
    }

    #[test]
    fn test_different_keys() {
        // Test different keys
        let key1 = AesSecret::generate_key();
        let key2 = AesSecret::generate_key();

        let secret1 = AesSecret { key: key1 };
        let secret2 = AesSecret { key: key2 };

        let data = b"Secret message";
        let encrypted = secret1.encrypt(data).unwrap();

        // Decryption should fail with a different key
        assert!(secret2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_invalid_data() {
        let key = AesSecret::generate_key();
        let secret = AesSecret { key };

        // Test invalid encrypted data
        let invalid_data = vec![1, 2, 3]; // Too short, missing salt and nonce
        assert!(secret.decrypt(&invalid_data).is_err());

        // Test corrupted encrypted data
        let data = b"Test message";
        let mut encrypted = secret.encrypt(data).unwrap();
        // Modify the last byte
        if let Some(last) = encrypted.last_mut() {
            *last ^= 0xff;
        }
        assert!(secret.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_key_consistency() {
        let key = AesSecret::generate_key();

        // Ensure different calls generate different keys
        let key2 = AesSecret::generate_key();
        assert_ne!(key, key2); // Random generated keys should be different
    }
}
