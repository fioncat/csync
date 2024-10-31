use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_addr")]
    pub addr: String,

    #[serde(default = "Config::default_password")]
    pub password: String,

    #[serde(default = "Config::default_download_dir")]
    pub download_dir: String,

    #[serde(default = "Config::default_upload_dir")]
    pub upload_dir: String,

    #[serde(default = "Config::default_client_interval")]
    pub client_interval: u64,

    #[serde(default = "Config::default_clipboard_interval")]
    pub clipboard_interval: u64,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let mut cfg = Self::load_raw(path)?;
        cfg.validate().context("validate config")?;
        Ok(cfg)
    }

    fn load_raw<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let path = match path {
            Some(path) => PathBuf::from(path.as_ref()),
            None => {
                let homedir = dirs::home_dir();
                if homedir.is_none() {
                    bail!("home directory is not supported in your system, please set config path manually");
                }

                homedir.unwrap().join(".config").join("csync.toml")
            }
        };

        match fs::read(&path) {
            Ok(data) => {
                let toml_str = String::from_utf8(data).with_context(|| {
                    format!("decode config file '{}' into utf-8", path.display())
                })?;

                let cfg: Config = toml::from_str(&toml_str)
                    .with_context(|| format!("parse config file '{}' toml", path.display()))?;

                Ok(cfg)
            }

            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),

            Err(err) => Err(err).with_context(|| format!("read config file '{}'", path.display())),
        }
    }

    fn validate(&mut self) -> Result<()> {
        if self.addr.is_empty() {
            bail!("config addr cannot be empty");
        }

        if self.password.is_empty() {
            bail!("config password cannot be empty");
        }

        Ok(())
    }

    fn default() -> Self {
        Self {
            addr: Self::default_addr(),
            password: Self::default_password(),
            download_dir: Self::default_download_dir(),
            upload_dir: Self::default_upload_dir(),
            client_interval: Self::default_client_interval(),
            clipboard_interval: Self::default_clipboard_interval(),
        }
    }

    fn default_addr() -> String {
        String::from("127.0.0.1:7703")
    }

    fn default_password() -> String {
        String::from("Csync_Password_123")
    }

    fn default_download_dir() -> String {
        match dirs::home_dir() {
            Some(home_dir) => home_dir.join("csync").to_string_lossy().to_string(),
            None => String::from("/tmp/csync"),
        }
    }

    fn default_upload_dir() -> String {
        let download_dir = PathBuf::from(Self::default_download_dir());
        download_dir.join("upload").to_string_lossy().to_string()
    }

    fn default_client_interval() -> u64 {
        300
    }

    fn default_clipboard_interval() -> u64 {
        300
    }
}
