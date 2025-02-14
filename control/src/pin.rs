use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::config::ConfigArgs;

use crate::ResourceType;

use super::RunCommand;

/// Update resource pin flag
#[derive(Args)]
pub struct PinArgs {
    /// Type of resource to display
    pub resource: ResourceType,

    /// The id to update pin flag
    pub id: u64,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for PinArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let name = self.resource.get_name();
        client.update_resource_pin(name, self.id).await?;

        Ok(())
    }
}
