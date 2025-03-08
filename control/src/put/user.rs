use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::user::PutUserRequest;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;

use crate::RunCommand;

/// Put a new user to the server
#[derive(Args)]
pub struct UserArgs {
    /// The user name
    pub name: String,

    /// The user password
    pub password: String,

    /// If set, the user will be an admin
    #[arg(short)]
    pub admin: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for UserArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        client
            .put_user(PutUserRequest {
                name: self.name.clone(),
                password: self.password.clone(),
                admin: self.admin,
            })
            .await?;

        Ok(())
    }
}
