use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use csync_misc::config::{CommonConfig, PathSet};
use csync_misc::dirs;
use csync_misc::logs::LogsConfig;
use log::info;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod};
use serde::{Deserialize, Serialize};

use crate::auth::jwt::{JwtTokenGenerator, JwtTokenValidator};
use crate::auth::rsa;
use crate::context::ServerContext;
use crate::db::config::DbConfig;
use crate::restful::RestfulServer;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_bind")]
    pub bind: String,

    #[serde(default)]
    pub ssl: bool,

    #[serde(default = "ServerConfig::default_admin_password")]
    pub admin_password: String,

    #[serde(default = "ServerConfig::default_recycle_hours")]
    pub recycle_hours: u64,

    #[serde(default = "ServerConfig::default_truncate_text_width")]
    pub truncate_text_width: usize,

    #[serde(default = "ServerConfig::default_salt_length")]
    pub salt_length: usize,

    #[serde(default)]
    pub db: DbConfig,

    pub keep_alive_secs: Option<u64>,

    pub workers: Option<u64>,

    pub payload_limit_mib: Option<u64>,

    #[serde(default = "ServerConfig::default_token_expiration_secs")]
    pub token_expiration_secs: u64,

    #[serde(default)]
    pub logs: LogsConfig,

    #[serde(skip)]
    pub recycle_seconds: u64,

    #[serde(skip)]
    pki_dir: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind: Self::default_bind(),
            ssl: false,
            admin_password: Self::default_admin_password(),
            recycle_hours: Self::default_recycle_hours(),
            truncate_text_width: Self::default_truncate_text_width(),
            salt_length: Self::default_salt_length(),
            db: DbConfig::default(),
            recycle_seconds: 0,
            keep_alive_secs: None,
            workers: None,
            payload_limit_mib: None,
            token_expiration_secs: Self::default_token_expiration_secs(),
            logs: LogsConfig::default(),
            pki_dir: PathBuf::new(),
        }
    }
}

impl CommonConfig for ServerConfig {
    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if self.bind.is_empty() {
            bail!("bind is required");
        }

        if self.admin_password.is_empty() {
            bail!("admin_password is required");
        }

        if self.recycle_hours == 0 {
            bail!("recycle_hours is required");
        }

        if self.truncate_text_width == 0 {
            bail!("truncate_text_width is required");
        }

        if self.truncate_text_width < Self::MIN_TRUNCATE_TEXT_WIDTH
            || self.truncate_text_width > Self::MAX_TRUNCATE_TEXT_WIDTH
        {
            bail!(
                "truncate_text_width must be in range [{}, {}]",
                Self::MIN_TRUNCATE_TEXT_WIDTH,
                Self::MAX_TRUNCATE_TEXT_WIDTH
            );
        }

        if self.salt_length == 0 {
            bail!("salt_length is required");
        }

        if self.salt_length < Self::MIN_SALT_LENGTH || self.salt_length > Self::MAX_SALT_LENGTH {
            bail!(
                "salt_length must be in range [{}, {}]",
                Self::MIN_SALT_LENGTH,
                Self::MAX_SALT_LENGTH
            );
        }

        self.db.complete(ps).context("db")?;

        if let Some(keep_alive_secs) = self.keep_alive_secs {
            if keep_alive_secs == 0 {
                bail!("keep_alive_secs must be greater than 0");
            }
        }

        if let Some(workers) = self.workers {
            if workers == 0 {
                bail!("workers must be greater than 0");
            }
        }

        if let Some(payload_limit_mib) = self.payload_limit_mib {
            if payload_limit_mib == 0 {
                bail!("payload_limit_mib must be greater than 0");
            }
        }

        if self.token_expiration_secs == 0 {
            bail!("token_expiration_secs is required");
        }
        if self.token_expiration_secs < Self::MIN_TOKEN_EXPIRATION_SECS
            || self.token_expiration_secs > Self::MAX_TOKEN_EXPIRATION_SECS
        {
            bail!(
                "token_expiration_secs must be in range [{}, {}]",
                Self::MIN_TOKEN_EXPIRATION_SECS,
                Self::MAX_TOKEN_EXPIRATION_SECS
            );
        }

        self.logs.complete(ps).context("logs")?;

        self.recycle_seconds = self.recycle_hours * 60 * 60;

        self.pki_dir = ps.config_dir.join("pki");
        dirs::ensure_dir_exists(&self.pki_dir).context("ensure pki dir")?;

        Ok(())
    }
}

impl ServerConfig {
    const MIN_TRUNCATE_TEXT_WIDTH: usize = 10;
    const MAX_TRUNCATE_TEXT_WIDTH: usize = 500;

    const MIN_SALT_LENGTH: usize = 8;
    const MAX_SALT_LENGTH: usize = 100;

    const MIN_TOKEN_EXPIRATION_SECS: u64 = 60;
    const MAX_TOKEN_EXPIRATION_SECS: u64 = 60 * 60 * 24 * 365;

    pub fn build_ctx(&self) -> Result<Arc<ServerContext>> {
        let db = self.db.build().context("init database")?;
        let (token_public, token_private) = self.read_jwt_keys()?;
        let jwt_generator = JwtTokenGenerator::new(&token_private, self.token_expiration_secs)
            .context("init jwt token generator")?;
        let jwt_validator =
            JwtTokenValidator::new(&token_public).context("init jwt token validator")?;

        let ctx = ServerContext {
            db,
            jwt_generator,
            jwt_validator,
            cfg: self.clone(),
            revision: Default::default(),
        };
        Ok(Arc::new(ctx))
    }

    pub fn build_restful_server(&self, ctx: Arc<ServerContext>) -> Result<RestfulServer> {
        let mut srv = RestfulServer::new(self.bind.clone(), ctx);
        if self.ssl {
            let ssl = self.build_ssl()?;
            srv.set_ssl(ssl);
        }

        if let Some(keep_alive_secs) = self.keep_alive_secs {
            srv.set_keep_alive_secs(keep_alive_secs);
        }

        if let Some(workers) = self.workers {
            srv.set_workers(workers);
        }

        if let Some(payload_limit_mib) = self.payload_limit_mib {
            srv.set_payload_limit_mib(payload_limit_mib);
        }

        Ok(srv)
    }

    fn read_jwt_keys(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let public_key_path = self.pki_dir.join("token_public.pem");
        let private_key_path = self.pki_dir.join("token_private.pem");
        if public_key_path.exists() && private_key_path.exists() {
            let public_key = fs::read(&public_key_path).context("read token public key")?;
            let private_key = fs::read(&private_key_path).context("read token private key")?;
            return Ok((public_key, private_key));
        }

        info!("Token keys for jwt not exists, try to generate new ones");
        let (public_key, private_key) =
            rsa::generate_rsa_keys().context("generate keys for token")?;

        fs::write(&public_key_path, &public_key).context("write token public key")?;
        fs::write(&private_key_path, &private_key).context("write token private key")?;

        Ok((public_key, private_key))
    }

    fn build_ssl(&self) -> Result<SslAcceptorBuilder> {
        let key_path = self.pki_dir.join("key.pem");
        if !key_path.exists() {
            bail!("ssl key file not exists: {:?}", key_path);
        }

        let cert_path = self.pki_dir.join("cert.pem");
        if !cert_path.exists() {
            bail!("ssl cert file not exists: {:?}", cert_path);
        }

        let mut builder =
            SslAcceptor::mozilla_intermediate(SslMethod::tls()).context("init ssl acceptor")?;

        builder
            .set_private_key_file(&key_path, openssl::ssl::SslFiletype::PEM)
            .context("load ssl key file")?;
        builder
            .set_certificate_chain_file(&cert_path)
            .context("load ssl cert file")?;

        Ok(builder)
    }

    fn default_bind() -> String {
        String::from("127.0.0.1:13577")
    }

    fn default_admin_password() -> String {
        String::from("admin_password123")
    }

    fn default_recycle_hours() -> u64 {
        24
    }

    fn default_truncate_text_width() -> usize {
        80
    }

    fn default_salt_length() -> usize {
        24
    }

    fn default_token_expiration_secs() -> u64 {
        60 * 60 // 1 hours
    }
}
