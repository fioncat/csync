use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{expandenv, CommonConfig, PathSet};

/// Secret related configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SecretConfig {
    /// Whether to enable Secret. When enabled, data will be encrypted and decrypted.
    /// If your clipboard data contains sensitive information, it is strongly recommended
    /// to enable Secret. This way, even if attackers obtain your user password, they
    /// cannot access the actual clipboard data. Internet service providers also cannot
    /// analyze your clipboard data through packet capture.
    /// Note that both server and client must enable Secret with matching keys,
    /// otherwise data will be rejected.
    /// Disabled by default.
    #[serde(default = "SecretConfig::default_enable")]
    pub enable: bool,

    /// Path to the key file. Only takes effect when the key is empty.
    /// If the file does not exist, we will automatically generate a random key.
    /// IMPORTANT: Keep this file secure and backed up. If the key is lost,
    /// encrypted data cannot be recovered. If the key is leaked, attackers
    /// can decrypt your clipboard data.
    /// Default is: {data_path}/pki/secret
    #[serde(default = "SecretConfig::default_key_path")]
    pub key_path: String,

    /// The key used for encryption and decryption. We use the symmetric encryption
    /// algorithm AES. The key length and format are arbitrary, but it is recommended
    /// to use a sufficiently long and complex key to ensure security.
    /// SECURITY WARNING: Protect this key carefully. If compromised, all encrypted
    /// clipboard data becomes accessible to attackers. Never share this key or
    /// store it in unsecured locations.
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
