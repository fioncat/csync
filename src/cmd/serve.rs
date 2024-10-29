use anyhow::Result;
use clap::Args;

use crate::logs;
use crate::net::Server;

/// Start the csync server
#[derive(Args)]
pub struct ServeArgs {
    /// The server bind address
    #[clap(short, long, default_value = "0.0.0.0:7703")]
    pub bind: String,

    /// The password uses to auth content
    #[clap(short, long, default_value = "Csync_Password_123")]
    pub password: String,

    /// The log level
    #[clap(short, long, default_value = "info")]
    pub level: String,
}

impl ServeArgs {
    pub async fn run(&self) -> Result<()> {
        logs::init(&self.level)?;
        let server = Server::bind(self.bind.clone(), self.password.clone()).await?;
        server.listen_and_serve().await
    }
}
