use anyhow::Result;
use clap::Args;

use crate::net::server;

/// Start the csync server
#[derive(Args)]
pub struct ServeArgs {
    /// The server bind address
    #[clap(short, long, default_value = "0.0.0.0:7703")]
    pub bind: String,

    /// The password uses to auth content
    #[clap(short, long)]
    pub password: Option<String>,
}

impl ServeArgs {
    pub async fn run(&self) -> Result<()> {
        server::start(&self.bind, self.password.as_deref()).await
    }
}
