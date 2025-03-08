use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::user::User;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::display;

use super::RunCommand;

/// Display info of the currently user.
#[derive(Args)]
pub struct WhoamiArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for WhoamiArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let user = cfg.username.clone();
        let user = if user == "admin" {
            User {
                name: "admin".to_string(),
                admin: true,
                update_time: 0,
            }
        } else {
            client.get_user(user).await?
        };

        display::pretty_json(user)?;

        Ok(())
    }
}
