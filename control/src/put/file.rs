use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Upload a file to the server, which can later be downloaded across different devices
/// using the read file command. This is an additional file-sharing feature provided by
/// csync, not a core function :). It should not be used for long-term file storage; the
/// file should be used promptly after uploading, as the server will delete it shortly.
#[derive(Args)]
pub struct FileArgs {
    /// The file needs to upload
    #[arg(short, long)]
    pub file: String,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for FileArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let path = PathBuf::from(&self.file);
        let name = match path.file_name() {
            Some(name) => match name.to_str() {
                Some(name) => name.to_string(),
                None => bail!("invalid file name"),
            },
            None => bail!("file name cannot be empty"),
        };

        let meta = fs::metadata(&self.file)?;
        let mode = meta.mode() as u32;

        let data = fs::read(&self.file).context("failed to read file")?;

        let file = client.put_file(name, mode, data).await?;

        println!("File id {}", file.id);
        Ok(())
    }
}
