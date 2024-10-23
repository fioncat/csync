use std::io::Cursor;

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, Nonce, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit};
use anyhow::Result;
use bytes::{Buf, Bytes};
use log::debug;
use pbkdf2::pbkdf2_hmac_array;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub const PROTOCOL_VERSION: u32 = 2;

const PROTOCOL_POST: u8 = b'0';
const PROTOCOL_GET: u8 = b'1';
const PROTOCOL_ERROR: u8 = b'2';

pub enum Frame {
    Post(DataFrame),
    Get,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataFrame {
    pub meta: MetadataFrame,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetadataFrame {
    pub device: String,
    pub file: Option<FileInfo>,
    pub auth: AuthInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub mode: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthInfo {
    pub nonce: Vec<u8>,
    pub salt: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("not enough data is available to parse a message")]
    Incomplete,

    #[error("invalid frame protocol: {0}")]
    Protocol(&'static str),

    #[error("authentication failed")]
    Auth,

    #[error("server error: {0}")]
    Server(String),

    #[error("unexpect next frame, expect to be `{0}`")]
    Unexpect(&'static str),
}

impl Frame {
    fn parse(src: &mut Cursor<&[u8]>, password: &str) -> Result<Option<(Self, usize)>, FrameError> {
        // The first step is to check if enough data has been buffered to parse
        // a single frame. This step is usually much faster than doing a full
        // parse of the frame, and allows us to skip allocating data structures
        // to hold the frame data unless we know the full frame has been
        // received.
        match Self::check(src) {
            Ok(_) => {
                // The `check` function will have advanced the cursor until the
                // end of the frame. Since the cursor had position set to zero
                // before `Frame::check` was called, we obtain the length of the
                // frame by checking the cursor position.
                let len = src.position() as usize;

                // Reset the position to zero before passing the cursor to
                // `Frame::parse`.
                src.set_position(0);

                // Parse the frame from the buffer. This allocates the necessary
                // structures to represent the frame and returns the frame
                // value.
                //
                // If the encoded frame representation is invalid, an error is
                // returned. This should terminate the **current** connection
                // but should not impact any other connected client.
                let frame = Self::parse_raw(src, password)?;

                Ok(Some((frame, len)))
            }

            // There is not enough data present in the read buffer to parse a
            // single frame. We must wait for more data to be received from the
            // socket. Reading from the socket will be done in the statement
            // after this `match`.
            //
            // We do not want to return `Err` from here as this "error" is an
            // expected runtime condition.
            Err(FrameError::Incomplete) => Ok(None),

            Err(err) => Err(err),
        }
    }

    fn parse_raw(src: &mut Cursor<&[u8]>, password: &str) -> Result<Self, FrameError> {
        let flag = get_u8(src)?;

        match flag {
            PROTOCOL_POST => {
                let meta: MetadataFrame = decode_object(&get_data(src)?)?;
                let raw_data = get_data(src)?;
                let data = meta.auth.decrypt(password, &raw_data)?;
                Ok(Self::Post(DataFrame { meta, data }))
            }

            PROTOCOL_GET => Ok(Self::Get),

            PROTOCOL_ERROR => {
                let data = get_data(src)?;
                let message = match String::from_utf8(data) {
                    Ok(msg) => msg,
                    Err(err) => {
                        debug!("[frame] decode server error message as utf-8 error: {err:#}");
                        return Err(FrameError::Protocol("invalid error message from server"));
                    }
                };
                Err(FrameError::Server(message))
            }

            _ => Err(Self::invalid_flag()),
        }
    }

    fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        let flag = get_u8(src)?;
        let check_rounds = match flag {
            PROTOCOL_POST => 2,
            PROTOCOL_GET => 0,
            PROTOCOL_ERROR => 1,
            _ => return Err(Self::invalid_flag()),
        };
        for _ in 0..check_rounds {
            check_data(src)?;
        }

        Ok(())
    }

    fn invalid_flag() -> FrameError {
        FrameError::Protocol("invalid frame flag")
    }

    async fn write<W>(&self, stream: &mut W, password: &str) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        match self {
            Self::Post(data_frame) => {
                stream.write_u8(PROTOCOL_POST).await?;
                write_data(stream, &encode_object(&data_frame.meta)?).await?;
                let data = data_frame.meta.auth.encrypt(password, &data_frame.data)?;
                write_data(stream, &data).await?;
            }

            Self::Get => stream.write_u8(PROTOCOL_GET).await?,

            Self::Error(message) => {
                stream.write_u8(PROTOCOL_ERROR).await?;
                write_data(stream, message.as_bytes()).await?;
            }
        }
        Ok(())
    }
}

impl AuthInfo {
    const KEY_LENGTH: usize = 32;
    const NONCE_LENGTH: usize = 12;
    const SALT_LENGTH: usize = 16;

    const PBKDF2_ROUNDS: u32 = 1024;
    const PBKDF2_ROUNDS_TEST: u32 = 128;

    pub fn new() -> Self {
        let mut rng = OsRng;
        Self {
            nonce: Self::generate_nonce(&mut rng),
            salt: Self::generate_salt(&mut rng),
        }
    }

    fn encrypt(&self, password: &str, data: &[u8]) -> Result<Vec<u8>, FrameError> {
        let raw_key = self.generate_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&raw_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(&self.nonce);
        match cipher.encrypt(nonce, data) {
            Ok(data) => Ok(data),
            Err(err) => {
                debug!("[frame] encrypt data error: {err:#}");
                Err(FrameError::Auth)
            }
        }
    }

    fn decrypt(&self, password: &str, data: &[u8]) -> Result<Vec<u8>, FrameError> {
        let raw_key = self.generate_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&raw_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(&self.nonce);
        match cipher.decrypt(nonce, data) {
            Ok(data) => Ok(data),
            Err(err) => {
                debug!("[frame] decrypt data error: {err:#}");
                Err(FrameError::Auth)
            }
        }
    }

    fn generate_key(&self, password: &str) -> [u8; Self::KEY_LENGTH] {
        if cfg!(test) {
            return pbkdf2_hmac_array::<Sha256, 32>(
                password.as_bytes(),
                &self.salt,
                Self::PBKDF2_ROUNDS_TEST,
            );
        }

        pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), &self.salt, Self::PBKDF2_ROUNDS)
    }

    fn generate_salt(rng: &mut OsRng) -> Vec<u8> {
        let mut salt = [0; Self::SALT_LENGTH];
        rng.fill_bytes(&mut salt);
        Vec::from(salt)
    }

    fn generate_nonce(rng: &mut OsRng) -> Vec<u8> {
        let mut nonce = [0; Self::NONCE_LENGTH];
        rng.fill_bytes(&mut nonce);
        Vec::from(nonce)
    }
}

fn check_data(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
    let len = get_decimal(src)? as usize;
    skip(src, len)?;
    Ok(())
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !src.has_remaining() {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u8())
}

fn get_data(src: &mut Cursor<&[u8]>) -> Result<Vec<u8>, FrameError> {
    let len = get_decimal(src)? as usize;
    if src.remaining() < len {
        return Err(FrameError::Incomplete);
    }
    let data = Bytes::copy_from_slice(&src.chunk()[..len]);
    skip(src, len)?;
    Ok(data.to_vec())
}

fn get_decimal(src: &mut Cursor<&[u8]>) -> Result<u64, FrameError> {
    if src.remaining() < 8 {
        return Err(FrameError::Incomplete);
    }
    Ok(src.get_u64())
}

fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), FrameError> {
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }
    src.advance(n);
    Ok(())
}

async fn write_data<W>(w: &mut W, data: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    w.write_u64(data.len() as u64).await?;
    w.write_all(data.as_ref()).await?;
    Ok(())
}

fn decode_object<T: DeserializeOwned>(data: &[u8]) -> Result<T, FrameError> {
    match bincode::deserialize(data) {
        Ok(v) => Ok(v),
        Err(_) => Err(FrameError::Protocol("decode frame failed, invalid object")),
    }
}

fn encode_object<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    match bincode::serialize(value) {
        Ok(data) => Ok(data),
        Err(_) => Err(FrameError::Protocol("encode frame failed")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth() {
        let cases = [
            ("password123", None, "data123"),
            ("password123", None, "A simple message"),
            ("password123", None, "A simple message"),
            ("password123", None, ""),
            ("", None, ""),
            ("password123", Some("password345"), "Wrong password"),
            ("", Some("password123"), "Wrong password"),
            ("password123", Some(""), "Wrong password"),
        ];

        for (password, wrong_password, data) in cases {
            let auth = AuthInfo::new();

            let cipher_data = auth.encrypt(password, data.as_bytes()).unwrap();
            assert_ne!(cipher_data, data.as_bytes());

            match wrong_password {
                Some(wrong_password) => {
                    let result = auth.decrypt(wrong_password, &cipher_data);
                    if result.is_ok() {
                        panic!("decrypt data should fail with wrong password");
                    }
                }
                None => {
                    let result = auth.decrypt(password, &cipher_data).unwrap();
                    assert_eq!(result, data.as_bytes());
                }
            }
        }
    }
}
