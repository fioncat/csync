use std::fs;
use std::io::{self, IsTerminal, Read, Write};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;

use crate::clipboard::Clipboard;
use crate::config::CommonConfig;
use crate::daemon::client::DaemonClient;
use crate::daemon::config::DaemonConfig;
use crate::humanize::human_bytes;
use crate::imghdr::is_data_image;

use super::{ConfigArgs, RunCommand};

/// Clipboard help command to write or read data to/from the clipboard
#[derive(Args)]
pub struct CbArgs {
    /// Write text to clipboard
    pub text: Option<String>,

    /// Write content from file to clipboard
    #[arg(short, long)]
    pub file: Option<String>,

    /// Write data to clipboard, without this, read data from clipboard
    #[arg(short, long)]
    pub write: bool,

    /// Sending request to daemon to write clipboard. This can avoid sending data to server.
    /// If you want to write something to clipboard but not send to server, you can use this
    /// option.
    #[arg(short, long)]
    pub daemon: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for CbArgs {
    async fn run(&self) -> Result<()> {
        if self.write {
            self.write().await
        } else {
            self.read()
        }
    }
}

impl CbArgs {
    fn read(&self) -> Result<()> {
        let cb = Clipboard::load()?;

        if cb.is_image()? {
            let data = match cb.read_image().context("read image")? {
                Some(data) => data,
                None => bail!("no image data in clipboard"),
            };
            if let Some(ref file) = self.file {
                fs::write(file, &data).context("write image file")?;
                return Ok(());
            }

            let mut stdout = io::stdout();
            let is_terminal = stdout.is_terminal();
            if is_terminal {
                bail!("cannot write image data to terminal, please use pipe");
            }
            stdout.write_all(&data).context("write image to stdout")?;
        } else if cb.is_text()? {
            let text = match cb.read_text().context("read text")? {
                Some(text) => text,
                None => bail!("no text data in clipboard"),
            };
            if let Some(ref file) = self.file {
                fs::write(file, &text).context("write text file")?;
                return Ok(());
            }
            print!("{text}");
        } else {
            bail!("clipboard does not contain text or image data");
        }

        Ok(())
    }

    async fn write(&self) -> Result<()> {
        let data = self.get_data()?;
        let size = human_bytes(data.len() as u64);

        if self.daemon {
            let ps = self.config.build_path_set()?;

            let cfg: DaemonConfig = ps.load_config("daemon", DaemonConfig::default)?;
            let client = DaemonClient::new(cfg.port);
            client
                .send_data(data)
                .await
                .context("send data to daemon")?;
            println!("Send {size} data to daemon server");
            return Ok(());
        }

        let cb = Clipboard::load()?;
        if is_data_image(&data) {
            cb.write_image(data).context("write image to clipboard")?;
            println!("Write {size} image data to clipboard",);
            Ok(())
        } else if let Ok(text) = String::from_utf8(data) {
            cb.write_text(text).context("write text to clipboard")?;
            println!("Write {size} text data to clipboard",);
            Ok(())
        } else {
            bail!("data is not valid utf-8 text or image");
        }
    }

    fn get_data(&self) -> Result<Vec<u8>> {
        if let Some(ref text) = self.text {
            return Ok(text.as_bytes().to_vec());
        }

        if let Some(ref file) = self.file {
            return fs::read(file).context("failed to read file");
        }

        let mut input = Vec::new();
        std::io::stdin()
            .read_to_end(&mut input)
            .context("read stdin")?;

        Ok(input)
    }
}
