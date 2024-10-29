use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::clipboard::Clipboard;
use crate::config::Config;
use crate::net::{Client, DataItem, FileItem};

/// Send data to server
#[derive(Args)]
pub struct SendArgs {
    /// Send data to server.
    pub data: Option<String>,

    /// Send file to server
    #[clap(long, short)]
    pub file: Option<String>,

    /// The config file to use.
    #[clap(long, short)]
    pub config: Option<String>,
}

impl SendArgs {
    pub async fn run(&self) -> Result<()> {
        let cfg = Config::load(self.config.as_deref())?;
        let mut client =
            Client::connect(cfg.addr.clone(), cfg.password.clone(), cfg.client_interval)
                .await
                .context("connect to server")?;

        let data_item = if let Some(data) = self.data.as_ref() {
            println!("Send {} bytes data from command arg", data.len());
            DataItem::Clipboard(data.as_bytes().to_vec())
        } else if let Some(file_path) = self.file.as_ref() {
            let meta = fs::metadata(file_path).context("read file metadata")?;
            let mode = meta.mode() as u64;
            let data = fs::read(file_path).context("read file")?;
            let path = PathBuf::from(file_path);
            println!("Send {} bytes data from file", data.len());
            let name = match path.file_name().context("get file name")?.to_str() {
                Some(name) => name.to_string(),
                None => bail!("invalid file name"),
            };
            DataItem::File(FileItem { name, mode, data })
        } else {
            let clipboard = Clipboard::build(cfg.clipboard_interval)?;
            let data = clipboard.read_raw().context("read data from clipboard")?;
            println!("Send {} bytes data from clipboard", data.len());
            DataItem::Clipboard(data)
        };

        client.send_data(data_item).await?;

        Ok(())
    }
}
