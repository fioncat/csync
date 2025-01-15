use std::{fs, io};

use anyhow::{Context, Result};
use log::{info, warn};
use sha2::{Digest, Sha256};

use super::aes::AesSecret;
use super::config::SecretConfig;

pub struct SecretFactory;

impl SecretFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn build_secret(&self, cfg: &SecretConfig) -> Result<Option<AesSecret>> {
        if !cfg.enable {
            warn!("Secret is disabled, your data may be exposed to public unsafely");
            return Ok(None);
        }

        if !cfg.key.is_empty() {
            let key = Sha256::digest(cfg.key.as_bytes()).to_vec();
            return Ok(Some(AesSecret::new(key)));
        }

        let key = match fs::read(&cfg.key_path) {
            Ok(key) => key,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                info!("Secret key file not found, generate a new one");
                let generated = AesSecret::generate_key();
                fs::write(&cfg.key_path, &generated).context("generate secret key")?;
                generated
            }
            Err(err) => return Err(err).context("read secret key"),
        };

        Ok(Some(AesSecret::new(key)))
    }
}
