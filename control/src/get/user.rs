use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::user::GetUserRequest;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

use super::QueryArgs;

/// Get users from server
#[derive(Args)]
pub struct UserArgs {
    /// The user name to query
    #[arg(short, long)]
    pub name: Option<String>,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub query: QueryArgs,
}

#[async_trait]
impl RunCommand for UserArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let query = self.query.build_query()?;
        let req = GetUserRequest {
            name: self.name.clone(),
            query,
        };

        let resp = client.get_users(req).await?;
        self.query.display_list(resp)?;

        Ok(())
    }
}
