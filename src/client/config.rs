use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{expandenv, CommonConfig, PathSet};
use crate::secret::config::SecretConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientConfig {
    #[serde(default = "ClientConfig::default_server")]
    pub server: String,

    #[serde(default = "ClientConfig::default_user")]
    pub user: String,

    #[serde(default = "ClientConfig::default_password")]
    pub password: String,

    #[serde(default = "ClientConfig::default_token_path")]
    pub token_path: String,

    #[serde(default = "ClientConfig::default_cert_path")]
    pub cert_path: String,

    #[serde(default = "SecretConfig::default")]
    pub secret: SecretConfig,
}

impl CommonConfig for ClientConfig {
    fn default() -> Self {
        Self {
            server: Self::default_server(),
            user: Self::default_user(),
            password: Self::default_password(),
            token_path: Self::default_token_path(),
            cert_path: Self::default_cert_path(),
            secret: SecretConfig::default(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        // We won't check server url is valid, the Client::new will check it.
        self.server = expandenv("server", &self.server)?;
        if self.server.is_empty() {
            bail!("server cannot be empty");
        }

        if self.server.starts_with("https") {
            self.cert_path = expandenv("cert_path", &self.cert_path)?;
            if self.cert_path.is_empty() {
                let path = ps.pki_path.join("cert.pem");
                self.cert_path = format!("{}", path.display());
            }
        }

        self.user = expandenv("user", &self.user)?;
        if self.user.is_empty() {
            bail!("user cannot be empty");
        }

        self.password = expandenv("password", &self.password)?;
        if self.password.is_empty() {
            bail!("password cannot be empty");
        }

        self.token_path = expandenv("token_path", &self.token_path)?;
        if self.token_path.is_empty() {
            let path = ps.data_path.join("token");
            self.token_path = format!("{}", path.display());
        }

        self.secret.complete(ps).context("secret")?;
        Ok(())
    }
}

impl ClientConfig {
    pub fn default_server() -> String {
        String::from("http://127.0.0.1:7881")
    }

    pub fn default_user() -> String {
        String::from("admin")
    }

    pub fn default_password() -> String {
        String::from("admin")
    }

    pub fn default_token_path() -> String {
        String::new()
    }

    pub fn default_cert_path() -> String {
        String::new()
    }
}
