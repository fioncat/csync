use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{bail, Context, Result};
use gethostname::gethostname;
use log::warn;
use serde::Deserialize;
use uuid::Uuid;

use crate::utils;

#[derive(Debug, Deserialize)]
pub struct Config {
    server: Option<String>,

    device: Option<String>,

    watch: Option<Vec<String>>,

    download_dir: Option<String>,

    password: Option<String>,

    read: Option<ReadConfig>,
    write: Option<WriteConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReadConfig {
    pub cmd: Option<Vec<String>>,

    pub interval: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WriteConfig {
    pub download_dir: Option<String>,

    pub text_cmd: Option<Vec<String>>,

    pub image_cmd: Option<Vec<String>>,
    pub download_image: Option<bool>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
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

            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                warn!(
                    "The config file '{}' not exists, use default config",
                    path.display()
                );
                Ok(Self {
                    server: None,
                    device: None,
                    watch: None,
                    download_dir: None,
                    password: None,
                    read: None,
                    write: None,
                })
            }

            Err(err) => Err(err).with_context(|| format!("read config file '{}'", path.display())),
        }
    }

    pub fn get_server(&self) -> &str {
        let addr = self.server.as_deref().unwrap_or_default();
        if !addr.is_empty() {
            return addr;
        }

        "127.0.0.1:7703"
    }

    pub fn get_device(&self) -> Cow<'_, str> {
        let device = self.device.as_deref().unwrap_or_default();
        if !device.is_empty() {
            return Cow::Borrowed(device);
        }

        let hostname = gethostname();
        if let Some(hostname) = hostname.to_str() {
            if !hostname.is_empty() {
                warn!("The device name is empty, use hostname '{hostname}' as device name");
                return Cow::Owned(String::from(hostname));
            }
        }

        let uuid = Uuid::new_v4();
        warn!("Both the device name and hostname is empty, use uuid '{uuid}' as device name");
        Cow::Owned(uuid.to_string())
    }

    pub fn get_watch(&self) -> Result<&[String]> {
        if let Some(watch) = self.watch.as_ref() {
            if !watch.is_empty() {
                return Ok(watch);
            }
        }

        bail!("The watch devices in config is empty, nothing to watch")
    }

    pub fn get_password(&self) -> Option<&str> {
        if let Some(password) = self.password.as_ref() {
            if !password.is_empty() {
                return Some(password);
            }
        }

        warn!("You are using csync without a password, your clipboard data will be exposed to network directly, please donot copy any sensitive data");
        None
    }

    #[inline]
    pub fn get_read(&mut self) -> Option<ReadConfig> {
        self.read.take()
    }

    #[inline]
    pub fn get_write(&mut self) -> WriteConfig {
        self.write.take().unwrap_or(WriteConfig {
            download_dir: None,
            text_cmd: None,
            image_cmd: None,
            download_image: None,
        })
    }
}
