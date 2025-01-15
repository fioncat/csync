pub mod aes;
pub mod config;
pub mod factory;

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as B64Engine;
use base64::Engine;

pub trait Secret {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
}

pub fn base64_encode(data: &[u8]) -> String {
    B64Engine.encode(data)
}

pub fn base64_decode(data: &str) -> Result<Vec<u8>> {
    B64Engine.decode(data).context("invalid base64 content")
}
