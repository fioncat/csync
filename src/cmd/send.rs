use anyhow::Result;
use clap::Args;

/// Send content to server
#[derive(Args)]
pub struct SendArgs {}

impl SendArgs {
    pub async fn run(&self) -> Result<()> {
        todo!()
    }
}
