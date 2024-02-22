use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{AeadMut, Nonce, OsRng};
use aes_gcm::{Aes256Gcm, Error as AesError, Key, KeyInit};
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;
use thiserror::Error;

use crate::frame::AuthFrame;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("the password is incorrect")]
    IncorrectPassword,

    #[error("aes256gcm failed: {0}")]
    AesError(AesError),
}

pub struct Auth {
    nonce: Vec<u8>,
    salt: Vec<u8>,

    key: [u8; Self::KEY_LENGTH],
}

impl Auth {
    const KEY_LENGTH: usize = 32;
    const NONCE_LENGTH: usize = 12;
    const CHECK_LENGTH: usize = 128;

    const PBKDF2_ROUNDS: u32 = 600_000;

    pub fn new<S: AsRef<str>>(password: S) -> Auth {
        let mut rng = OsRng;
        let nonce = Self::generate_nonce(&mut rng);
        let salt = Self::generate_salt(&mut rng);
        let key = Self::generate_key(password, &salt);

        Auth { nonce, salt, key }
    }

    pub fn from_frame<S: AsRef<str>>(password: S, frame: AuthFrame) -> Result<Auth, AuthError> {
        let AuthFrame {
            nonce,
            salt,
            check,
            check_plain,
        } = frame;

        let key = Self::generate_key(password, &salt);
        let auth = Auth { nonce, salt, key };

        let check_result = auth.decrypt(&check)?;
        if check_result != check_plain {
            return Err(AuthError::IncorrectPassword);
        }

        Ok(auth)
    }

    pub fn build_frame(&self) -> Result<AuthFrame, AuthError> {
        let mut rng = OsRng;
        let check_plain = Self::generate_check_plain(&mut rng);
        let check = self.encrypt(&check_plain)?;

        Ok(AuthFrame {
            nonce: self.nonce.clone(),
            salt: self.salt.clone(),
            check,
            check_plain,
        })
    }

    pub fn encrypt(&self, plain_data: &[u8]) -> Result<Vec<u8>, AuthError> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let mut cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(&self.nonce);
        match cipher.encrypt(nonce, plain_data) {
            Ok(data) => Ok(data),
            Err(err) => Err(AuthError::AesError(err)),
        }
    }

    pub fn decrypt(&self, cipher_data: &[u8]) -> Result<Vec<u8>, AuthError> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let mut cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(&self.nonce);
        match cipher.decrypt(nonce, cipher_data) {
            Ok(data) => Ok(data),
            Err(_) => Err(AuthError::IncorrectPassword),
        }
    }

    #[inline]
    fn generate_key<S: AsRef<str>>(password: S, salt: &[u8]) -> [u8; Self::KEY_LENGTH] {
        pbkdf2_hmac_array::<Sha256, 32>(password.as_ref().as_bytes(), salt, Self::PBKDF2_ROUNDS)
    }

    #[inline]
    fn generate_check_plain(rng: &mut OsRng) -> Vec<u8> {
        let mut check_plain = [0; Self::CHECK_LENGTH];
        rng.fill_bytes(&mut check_plain);
        Vec::from(check_plain)
    }

    #[inline]
    fn generate_salt(rng: &mut OsRng) -> Vec<u8> {
        let mut salt = [0; Self::CHECK_LENGTH];
        rng.fill_bytes(&mut salt);
        Vec::from(salt)
    }

    #[inline]
    fn generate_nonce(rng: &mut OsRng) -> Vec<u8> {
        let mut nonce = [0; Self::NONCE_LENGTH];
        rng.fill_bytes(&mut nonce);
        Vec::from(nonce)
    }
}
