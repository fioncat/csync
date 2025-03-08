use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::user::PatchUserRequest;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Patch a user
#[derive(Args)]
pub struct UserArgs {
    /// The user name to patch
    pub name: String,

    /// Update user password
    #[arg(long, short)]
    pub password: Option<String>,

    /// Set user as admin
    #[arg(long)]
    pub admin: bool,

    #[arg(long)]
    pub no_admin: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for UserArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let admin = if self.admin {
            Some(true)
        } else if self.no_admin {
            Some(false)
        } else {
            None
        };

        let patch = PatchUserRequest {
            name: self.name.clone(),
            password: self.password.clone(),
            admin,
        };

        client.patch_user(patch).await?;

        Ok(())
    }
}
