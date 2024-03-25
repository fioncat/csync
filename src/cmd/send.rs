use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::config::Config;
use crate::net::client::SendClient;
use crate::net::frame::DataFrameInfo;
use crate::net::frame::{DataFrame, FileInfo};
use crate::utils;

/// Send content to server
#[derive(Args)]
pub struct SendArgs {
    /// Send text
    pub text: Option<String>,

    /// Send file
    #[clap(long, short)]
    pub file: Option<String>,

    /// The config file to use.
    #[clap(long, short)]
    pub config: Option<String>,
}

impl SendArgs {
    pub async fn run(&self) -> Result<()> {
        let cfg = Config::load(self.config.as_deref())?;

        let (data, file_info) = self.get_data()?;
        let digest = utils::get_digest(&data);

        let frame = DataFrame {
            info: DataFrameInfo {
                device: Some(cfg.device.clone()),
                digest,
                file: file_info,
            },
            body: data,
        };

        println!("Send {} data to server", frame.body.len());
        let mut client =
            SendClient::connect(&cfg.server, &cfg.device, cfg.password.as_deref()).await?;
        client.send(&frame).await?;

        Ok(())
    }

    fn get_data(&self) -> Result<(Vec<u8>, Option<FileInfo>)> {
        if self.file.is_some() && self.text.is_some() {
            bail!("the file and text args cannot be both provided");
        }

        if let Some(text) = self.text.as_ref() {
            return Ok((text.clone().into_bytes(), None));
        }

        if let Some(path) = self.file.as_ref() {
            let path = PathBuf::from(path);
            let meta = fs::metadata(&path)
                .with_context(|| format!("get metadata for file '{}'", path.display()))?;

            let mode = meta.mode() as u64;
            let name = path.file_name().unwrap_or_default();
            if name.is_empty() {
                bail!("invalid path '{}'", path.display());
            }

            let info = FileInfo {
                name: name.to_string_lossy().into_owned(),
                mode,
            };

            let data =
                fs::read(&path).with_context(|| format!("read file '{}'", path.display()))?;

            return Ok((data, Some(info)));
        }

        let mut buf = Vec::with_capacity(512);
        io::stdin()
            .lock()
            .read_to_end(&mut buf)
            .context("read data from stdin")?;
        Ok((buf, None))
    }
}
