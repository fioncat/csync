use std::borrow::Cow;
use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Args;
use log::info;
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio::time::{self, Instant};

use crate::config::Config;
use crate::net::client::SendClient;
use crate::net::frame::{DataFrame, DataFrameInfo, FileInfo};
use crate::utils::Cmd;

/// Send content to server
#[derive(Args)]
pub struct SendArgs {
    /// Send text to server
    pub text: Option<String>,

    /// Send file to server
    #[clap(long, short)]
    pub file: Option<String>,

    /// Watch the command `WATCH_CMD` and send changes to server
    #[clap(long, short)]
    pub watch: bool,

    /// The config file to use. Default is `~/.config/csync.toml`
    #[clap(long, short)]
    pub config: Option<String>,

    /// The command to watch, commonly is the clipboard command, like `xclip`, `wl-copy`,
    /// `pbcopy`, etc.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub watch_cmd: Vec<String>,
}

impl SendArgs {
    pub async fn run(&self) -> Result<()> {
        let cfg = Config::load(self.config.as_deref()).context("load config")?;
        if self.watch {
            if self.watch_cmd.is_empty() {
                bail!("in watch mode, the watch command cannot be empty, please refer to command usage");
            }

            return self.watch(&cfg).await;
        }

        if !self.watch_cmd.is_empty() {
            bail!("the watch command should be only provided in watch mode, please refer to command usage");
        }

        self.send(&cfg).await
    }

    async fn watch(&self, cfg: &Config) -> Result<()> {
        let (mut send_client, device) = self.connect_server(cfg).await?;

        let mut info = DataFrameInfo {
            device: Some(device.into_owned()),
            digest: String::new(),
            file: None,
        };

        let mut intv = time::interval_at(Instant::now(), Duration::from_millis(200));

        loop {
            intv.tick().await;

            let mut cmd = Cmd::new(&self.watch_cmd, None, true);
            let output = cmd.execute().context("execute watch command")?;
            if output.is_none() {
                continue;
            }

            let data = output.unwrap();
            let digest = self.get_digest(&data);
            if digest == info.digest {
                continue;
            }

            let frame = DataFrame {
                // TODO: Here we can use `Cow` to save this `clone`.
                info: info.clone(),
                body: data,
            };

            send_client
                .send(&frame)
                .await
                .context("send data to server")?;
            info.digest = digest;
        }
    }

    async fn send(&self, cfg: &Config) -> Result<()> {
        let (mut send_client, device) = self.connect_server(cfg).await?;

        let (file_info, data) = self.get_data()?;
        let digest = self.get_digest(&data);
        let data_len = data.len();

        let data_frame = DataFrame {
            info: DataFrameInfo {
                device: Some(device.into_owned()),
                digest,
                file: file_info,
            },
            body: data,
        };

        send_client
            .send(&data_frame)
            .await
            .context("send data to server")?;
        info!("Send {data_len} data to server done");

        Ok(())
    }

    async fn connect_server<'a>(
        &self,
        cfg: &'a Config,
    ) -> Result<(SendClient<TcpStream>, Cow<'a, str>)> {
        let device = cfg.get_device();
        let addr = cfg.get_server();
        let password = cfg.get_password();
        let send_client = SendClient::connect(addr, device.as_ref(), password)
            .await
            .context("connect to server")?;
        Ok((send_client, device))
    }

    fn get_data(&self) -> Result<(Option<FileInfo>, Vec<u8>)> {
        if self.file.is_some() && self.text.is_some() {
            bail!("the file and text args cannot be both provided, which one should I use?");
        }

        if let Some(path) = self.file.as_ref() {
            let meta = fs::metadata(path).with_context(|| format!("stat file '{path}'"))?;
            if !meta.is_file() {
                bail!("the path '{path}' is not a file");
            }

            let path_buf = PathBuf::from(path);
            let name = path_buf.file_name().unwrap_or_default();
            let name = name.to_str().unwrap_or_default();
            if name.is_empty() {
                bail!("invalid file name for path '{path}'");
            }

            // TODO: Support Windows
            let mode = meta.mode() as u64;

            let data = fs::read(path).with_context(|| format!("read file '{path}'"))?;
            let info = FileInfo {
                name: String::from(name),
                mode,
            };

            return Ok((Some(info), data));
        }

        if let Some(text) = self.text.as_ref() {
            return Ok((None, text.clone().into_bytes()));
        }

        let mut buf = Vec::with_capacity(512);
        io::stdin()
            .lock()
            .read_to_end(&mut buf)
            .context("read data from stdin")?;
        Ok((None, buf))
    }

    fn get_digest(&self, data: &[u8]) -> String {
        let mut hash = Sha256::new();
        hash.update(data);
        let result = hash.finalize();
        format!("{:x}", result)
    }
}
