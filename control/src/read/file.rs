use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::OpenOptionsExt;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::config::ConfigArgs;
use csync_misc::humanize::human_bytes;

use crate::RunCommand;

/// Read the contents of a file from the server.
#[derive(Args)]
pub struct FileArgs {
    /// Specify the file ID to read; if not provided, the latest file will be read.
    pub id: Option<u64>,

    /// Download the file from the server to a local path. If this option is not provided,
    /// it will attempt to output the content to stdout.
    #[arg(short, long)]
    pub file: Option<String>,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for FileArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let (info, data) = match self.id {
            Some(id) => client.read_file(id).await?,
            None => client.read_latest_file().await?,
        };
        if let Some(ref path) = self.file {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(info.mode)
                .open(path)
                .context("failed to open file")?;
            file.write_all(&data).context("failed to write file")?;

            println!(
                "Write {} data to file: '{path}' done",
                human_bytes(info.size)
            );
            return Ok(());
        }

        let mut stdout = io::stdout();
        let is_terminal = stdout.is_terminal();

        if is_terminal {
            let str = match String::from_utf8(data.clone()) {
                Ok(str) => str,
                Err(_) => bail!("file data is not valid utf8, cannot be printed to terminal"),
            };
            print!("{str}");
            return Ok(());
        }

        stdout.write_all(&data).context("write stdout")?;
        Ok(())
    }
}
