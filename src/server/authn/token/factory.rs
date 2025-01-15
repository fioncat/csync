use std::{fs, io};

use anyhow::{Context, Result};
use log::info;

use crate::rsa::generate_rsa_keys;

use super::config::TokenConfig;
use super::jwt::{JwtTokenGenerator, JwtTokenValidator};

pub struct TokenFactory {
    public_key: Vec<u8>,
    private_key: Vec<u8>,
    expiry: u64,
}

impl TokenFactory {
    pub fn new(cfg: &TokenConfig) -> Result<Self> {
        let (public_key, private_key) = match fs::read(&cfg.public_key_path) {
            Ok(data) => (data, None),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                info!("Public key and private key for token not found, generating with rsa");
                let (public_key, private_key) = generate_rsa_keys()?;
                fs::write(&cfg.public_key_path, &public_key)?;
                fs::write(&cfg.private_key_path, &private_key)?;
                (public_key, Some(private_key))
            }
            Err(err) => return Err(err).context("read token public key failed"),
        };

        let private_key = match private_key {
            Some(key) => key,
            None => fs::read(&cfg.private_key_path)?,
        };

        Ok(Self {
            public_key,
            private_key,
            expiry: cfg.expiry,
        })
    }

    pub fn build_token_generator(&self) -> Result<JwtTokenGenerator> {
        JwtTokenGenerator::new(&self.private_key, self.expiry)
    }

    pub fn build_token_validator(&self) -> Result<JwtTokenValidator> {
        JwtTokenValidator::new(&self.public_key)
    }
}
