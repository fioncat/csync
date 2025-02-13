use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::config::ConfigArgs;

use super::{ResourceType, RunCommand};

/// Delete a resource from the server.
#[derive(Args)]
pub struct DeleteArgs {
    /// Type of resource to delete.
    pub resource: ResourceType,

    /// ID of the resource to delete.
    pub id: String,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for DeleteArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        client
            .delete_resource(self.resource.get_name(), &self.id)
            .await?;
        Ok(())
    }
}
