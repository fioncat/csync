use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{expandenv, CommonConfig, PathSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SecretConfig {
    #[serde(default = "SecretConfig::default_enable")]
    pub enable: bool,

    #[serde(default = "SecretConfig::default_key_path")]
    pub key_path: String,

    #[serde(default = "SecretConfig::default_key")]
    pub key: String,
}

impl CommonConfig for SecretConfig {
    fn default() -> Self {
        Self {
            enable: Self::default_enable(),
            key_path: Self::default_key_path(),
            key: Self::default_key(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if !self.enable {
            return Ok(());
        }

        self.key_path = expandenv("key_path", &self.key_path)?;
        if self.key_path.is_empty() {
            let path = ps.pki_path.join("secret");
            self.key_path = format!("{}", path.display());
        }

        self.key = expandenv("key", &self.key)?;

        Ok(())
    }
}

impl SecretConfig {
    pub fn default_enable() -> bool {
        false
    }

    pub fn default_key_path() -> String {
        String::new()
    }

    pub fn default_key() -> String {
        String::new()
    }
}
