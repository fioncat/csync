use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::utils;

mod defaults;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "defaults::server")]
    pub server: String,

    #[serde(default = "defaults::device")]
    pub device: String,

    #[serde(default = "defaults::empty_vec")]
    pub watch: Vec<String>,

    #[serde(default = "defaults::work_dir")]
    pub work_dir: String,

    pub read: Option<ReadConfig>,
    pub write: Option<WriteConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ReadConfig {
    #[serde(default = "defaults::empty_vec")]
    pub cmd: Vec<String>,

    #[serde(default = "defaults::read_interval")]
    pub interval: u32,

    #[serde(default = "defaults::disable")]
    pub notify: bool,
}

#[derive(Debug, Deserialize)]
pub struct WriteConfig {
    #[serde(default = "defaults::empty_vec")]
    pub text_cmd: Vec<String>,

    #[serde(default = "defaults::empty_vec")]
    pub image_cmd: Vec<String>,

    #[serde(default = "defaults::disable")]
    pub download_image: bool,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let mut cfg = Self::_load(path)?;
        cfg.validate().context("validate config")?;
        Ok(cfg)
    }

    fn _load<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
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

    #[inline]
    pub fn get_work_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir)
    }

    #[inline]
    pub fn get_notify_path(&self) -> PathBuf {
        self.get_work_dir().join("notify")
    }

    #[inline]
    pub fn get_image_path(&self) -> PathBuf {
        self.get_work_dir().join("image.png")
    }

    #[inline]
    pub fn get_download_dir(&self) -> Result<PathBuf> {
        let dir = self.get_work_dir().join("download");
        utils::ensure_dir(&dir).context("ensure download dir")?;
        Ok(dir)
    }

    #[inline]
    fn default() -> Self {
        Self {
            server: defaults::server(),
            device: defaults::device(),
            watch: defaults::empty_vec(),
            work_dir: defaults::work_dir(),
            read: None,
            write: None,
        }
    }

    fn validate(&mut self) -> Result<()> {
        if self.server.is_empty() {
            bail!("config server cannot be empty");
        }

        if self.device.is_empty() {
            bail!("config device cannot be empty");
        }

        self.work_dir = utils::shellexpand(&self.work_dir).context("expand env for work_dir")?;
        if self.work_dir.is_empty() {
            bail!("config work_dir cannot be empty");
        }
        utils::ensure_dir(&self.work_dir).context("ensure work_dir")?;

        if let Some(read) = self.read.as_ref() {
            read.validate()?;
        }

        Ok(())
    }
}

impl ReadConfig {
    fn validate(&self) -> Result<()> {
        if self.cmd.is_empty() && !self.notify {
            bail!("read cmd and notify cannot be both empty");
        }

        if self.interval < 100 || self.interval > 5000 {
            bail!(
                "read interval should be in range [100,5000], found {}",
                self.interval
            );
        }

        Ok(())
    }
}
