use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::{expandenv, CommonConfig, PathSet};
use crate::secret::config::SecretConfig;

use super::authn::config::AuthnConfig;
use super::authz::config::AuthzConfig;
use super::db::config::DbConfig;
use super::recycle::config::RecycleConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_bind")]
    pub bind: String,

    #[serde(default = "ServerConfig::default_ssl")]
    pub ssl: bool,

    #[serde(default = "ServerConfig::default_cert_path")]
    pub cert_path: String,

    #[serde(default = "ServerConfig::default_key_path")]
    pub key_path: String,

    #[serde(default = "ServerConfig::default_allow_insecure_client")]
    pub allow_insecure_client: bool,

    #[serde(default = "ServerConfig::default_keep_alive_secs")]
    pub keep_alive_secs: u64,

    #[serde(default = "ServerConfig::default_workers")]
    pub workers: u64,

    #[serde(default = "ServerConfig::default_payload_limit_mib")]
    pub payload_limit_mib: usize,

    #[serde(default = "AuthnConfig::default")]
    pub authn: AuthnConfig,

    #[serde(default = "AuthzConfig::default")]
    pub authz: AuthzConfig,

    #[serde(default = "DbConfig::default")]
    pub db: DbConfig,

    #[serde(default = "SecretConfig::default")]
    pub secret: SecretConfig,

    #[serde(default = "RecycleConfig::default")]
    pub recycle: RecycleConfig,
}

impl CommonConfig for ServerConfig {
    fn default() -> Self {
        Self {
            bind: Self::default_bind(),
            ssl: Self::default_ssl(),
            cert_path: Self::default_cert_path(),
            key_path: Self::default_key_path(),
            allow_insecure_client: Self::default_allow_insecure_client(),
            keep_alive_secs: Self::default_keep_alive_secs(),
            workers: Self::default_workers(),
            payload_limit_mib: Self::default_payload_limit_mib(),
            authn: AuthnConfig::default(),
            authz: AuthzConfig::default(),
            db: DbConfig::default(),
            secret: SecretConfig::default(),
            recycle: RecycleConfig::default(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        self.bind = expandenv("bind", &self.bind)?;
        if self.bind.is_empty() {
            bail!("bind cannot be empty");
        }

        self.cert_path = expandenv("cert_path", &self.cert_path)?;
        if self.cert_path.is_empty() {
            let path = ps.pki_path.join("server.crt");
            self.cert_path = format!("{}", path.display());
        }

        self.key_path = expandenv("key_path", &self.key_path)?;
        if self.key_path.is_empty() {
            let path = ps.pki_path.join("server.key");
            self.key_path = format!("{}", path.display());
        }

        if self.payload_limit_mib < Self::MIN_PAYLOAD_LIMIT_MIB {
            bail!(
                "payload_limit_mib must be greater than or equal to {}",
                Self::MIN_PAYLOAD_LIMIT_MIB
            );
        }
        if self.payload_limit_mib > Self::MAX_PAYLOAD_LIMIT_MIB {
            bail!(
                "payload_limit_mib must be less than or equal to {}",
                Self::MAX_PAYLOAD_LIMIT_MIB
            );
        }

        self.authn.complete(ps).context("authn")?;
        self.authz.complete(ps).context("authz")?;
        self.db.complete(ps).context("db")?;
        self.secret.complete(ps).context("secret")?;

        Ok(())
    }
}

impl ServerConfig {
    const MAX_PAYLOAD_LIMIT_MIB: usize = 10;
    const MIN_PAYLOAD_LIMIT_MIB: usize = 1;

    pub fn default_bind() -> String {
        String::from("127.0.0.1:7881")
    }

    pub fn default_ssl() -> bool {
        false
    }

    pub fn default_cert_path() -> String {
        String::new()
    }

    pub fn default_key_path() -> String {
        String::new()
    }

    pub fn default_allow_insecure_client() -> bool {
        false
    }

    pub fn default_keep_alive_secs() -> u64 {
        0
    }

    pub fn default_workers() -> u64 {
        0
    }

    pub fn default_payload_limit_mib() -> usize {
        3
    }
}
