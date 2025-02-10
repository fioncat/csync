use std::path::PathBuf;
use std::{env, fs, io};

use anyhow::{bail, Context, Result};
use log::warn;
use serde::de::DeserializeOwned;

use crate::dirs::ensure_dir_exists;

pub struct PathSet {
    pub config_path: PathBuf,
    pub data_path: PathBuf,
    pub pki_path: PathBuf,
}

impl PathSet {
    pub fn new(config_path: Option<PathBuf>, data_path: Option<PathBuf>) -> Result<Self> {
        // Check if running as root (UID == 0)
        let is_root = unsafe { libc::geteuid() == 0 };

        // Determine config path
        let config_path = if let Some(path) = config_path {
            path
        } else if let Ok(path) = env::var("CSYNC_CONFIG") {
            PathBuf::from(path)
        } else if is_root {
            PathBuf::from("/etc/csync")
        } else {
            Self::home_dir()?.join(".config").join("csync")
        };

        // Determine data path
        let data_path = if let Some(path) = data_path {
            path
        } else if let Ok(path) = env::var("CSYNC_DATA") {
            PathBuf::from(path)
        } else if is_root {
            PathBuf::from("/var/lib/csync")
        } else {
            Self::home_dir()?.join(".local").join("share").join("csync")
        };

        // PKI path is always under config path
        let pki_path = config_path.join("pki");

        // Ensure all directories exist
        ensure_dir_exists(&config_path)
            .with_context(|| format!("ensure config directory: {}", config_path.display()))?;
        ensure_dir_exists(&data_path)
            .with_context(|| format!("ensure data directory: {}", data_path.display()))?;
        ensure_dir_exists(&pki_path)
            .with_context(|| format!("ensure pki directory: {}", pki_path.display()))?;

        Ok(Self {
            config_path,
            data_path,
            pki_path,
        })
    }

    pub fn load_config<T, F>(&self, name: &str, default_func: F) -> Result<T>
    where
        T: CommonConfig + DeserializeOwned,
        F: FnOnce() -> T,
    {
        let path = self.config_path.join(format!("{name}.toml"));
        let mut cfg: T = match fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).context("parse config toml")?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                warn!("Config file for {name} not found, using defaults");
                default_func()
            }
            Err(err) => {
                return Err(err).context(format!("read config file: {}", path.display()));
            }
        };

        cfg.complete(self).context("validate config")?;
        Ok(cfg)
    }

    fn home_dir() -> Result<PathBuf> {
        let dir = std::env::var_os("HOME") // Unix/Linux/macOS
            .or_else(|| std::env::var_os("USERPROFILE")) // Windows
            .map(PathBuf::from);
        match dir {
            Some(dir) => Ok(dir),
            None => {
                bail!("could not determine home directory, please specify config path manually")
            }
        }
    }
}

pub trait CommonConfig {
    fn default() -> Self;
    fn complete(&mut self, ps: &PathSet) -> Result<()>;
}

/// See: [`shellexpand::full`].
pub fn expandenv(name: &str, s: impl AsRef<str>) -> Result<String> {
    let s =
        shellexpand::full(s.as_ref()).with_context(|| format!("expand env value for '{name}'"))?;
    Ok(s.to_string())
}
