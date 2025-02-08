use anyhow::{bail, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::client::factory::ClientFactory;
use csync_misc::types::cmd::ConfigArgs;
use csync_misc::types::user::{Role, User};

use crate::RunCommand;

/// Create a new user or update an existing user on the server.
#[derive(Args)]
pub struct UserArgs {
    /// Username, which must be unique on the server. If the user does not exist, it will be
    /// created.
    pub name: String,

    /// User password, which is required if creating a user. If updating a user and this
    /// option is provided, it will update the specified user's password.
    #[arg(short, long)]
    pub password: Option<String>,

    /// User roles, which is required if creating a user. If updating a user and this option
    /// is provided, it will update the specified user's roles.
    #[arg(short, long)]
    pub roles: Option<Vec<String>>,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for UserArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let mut roles = vec![];
        if let Some(role_names) = &self.roles {
            for role_name in role_names.iter() {
                roles.push(Role {
                    name: role_name.clone(),
                    rules: vec![],
                    create_time: 0,
                    update_time: 0,
                });
            }
        }

        if self.password.is_none() && roles.is_empty() {
            bail!("either password or roles must be provided");
        }

        let user = User {
            name: self.name.clone(),
            roles,
            password: self.password.clone(),
            create_time: 0,
            update_time: 0,
        };
        client.put_user(&user).await?;

        Ok(())
    }
}
