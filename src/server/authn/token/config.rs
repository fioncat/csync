use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::config::{expandenv, CommonConfig, PathSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenConfig {
    #[serde(default = "TokenConfig::default_key_path")]
    pub public_key_path: String,

    #[serde(default = "TokenConfig::default_key_path")]
    pub private_key_path: String,

    #[serde(default = "TokenConfig::default_expiry")]
    pub expiry: u64,

    #[serde(skip)]
    pub generate_if_not_exists: bool,
}

impl CommonConfig for TokenConfig {
    fn default() -> Self {
        Self {
            public_key_path: Self::default_key_path(),
            private_key_path: Self::default_key_path(),
            expiry: Self::default_expiry(),
            generate_if_not_exists: false,
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if self.expiry == 0 {
            bail!("token expiry should not be 0");
        }

        self.public_key_path = expandenv("public_key_path", &self.public_key_path)?;
        self.private_key_path = expandenv("private_key_path", &self.private_key_path)?;

        if self.public_key_path.is_empty() && self.private_key_path.is_empty() {
            self.generate_if_not_exists = true;

            let path = ps.pki_path.join("token_public.pem");
            self.public_key_path = format!("{}", path.display());

            let path = ps.pki_path.join("token_private.pem");
            self.private_key_path = format!("{}", path.display());

            return Ok(());
        }

        if !self.public_key_path.is_empty() && !self.private_key_path.is_empty() {
            return Ok(());
        }

        bail!("both public_key_path and private_key_path should be set or both should be empty")
    }
}

impl TokenConfig {
    pub fn default_key_path() -> String {
        String::new()
    }

    pub fn default_expiry() -> u64 {
        60 * 60 // 60 minutes
    }
}
