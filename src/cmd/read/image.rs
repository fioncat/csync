use std::fs;
use std::io::{self, IsTerminal, Write};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;

use crate::client::factory::ClientFactory;
use crate::clipboard::Clipboard;
use crate::cmd::{ConfigArgs, RunCommand};

/// Read the content of an image from the server.
#[derive(Args)]
pub struct ImageArgs {
    /// Specify the image ID to read; if not provided, the latest image will be read.
    pub id: Option<u64>,

    /// Write the image data to a file. If this option is not provided, it will attempt to
    /// output the content to stdout (can't be output to terminal).
    #[arg(short, long)]
    pub file: Option<String>,

    /// Write the image directly to the clipboard.
    #[arg(short, long)]
    pub cb: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for ImageArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let mut stdout = io::stdout();
        let is_terminal = stdout.is_terminal();
        if self.file.is_none() && is_terminal {
            bail!("cannot write image data to terminal, please use pipe");
        }

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let data = match self.id {
            Some(id) => client.read_image(id).await?,
            None => client.read_latest_image().await?,
        };

        if self.cb {
            let cb = Clipboard::load()?;
            cb.write_image(data).context("write image to clipboard")?;
            return Ok(());
        }

        if let Some(ref file) = self.file {
            return fs::write(file, data).context("failed to write file");
        }

        stdout.write_all(&data).context("write stdout")?;
        Ok(())
    }
}
