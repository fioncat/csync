use anyhow::Result;
use clap::Args;

/// Watch and receive content from server
#[derive(Args)]
pub struct WatchArgs {}

impl WatchArgs {
    pub async fn run(&self) -> Result<()> {
        todo!()
    }
}
