use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::types::cmd::ConfigArgs;

use super::{ResourceType, RunCommand};

/// Check if the currently authenticated user can perform a specific operation.
#[derive(Args)]
pub struct CaniArgs {
    /// Operation to check, available: get, list, put, delete.
    pub verb: String,

    /// Resource type to check.
    pub resource: ResourceType,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for CaniArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let allow = client.cani(&self.verb, self.resource.get_name()).await?;
        println!("{allow}");

        Ok(())
    }
}
