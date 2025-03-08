use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::metadata::GetMetadataRequest;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;
use csync_misc::time;

use crate::RunCommand;

use super::QueryArgs;

/// Get blob metadatas from server
#[derive(Args)]
pub struct MetadataArgs {
    /// The blob id to query
    #[arg(short, long)]
    pub id: Option<u64>,

    /// The owner to query
    #[arg(long)]
    pub owner: Option<String>,

    /// The sha256 to query
    #[arg(long)]
    pub sha256: Option<String>,

    /// Query blob recycle after this time
    #[arg(short, long)]
    pub recycle: Option<String>,

    /// Print blobs in summary format, "{ID}. {SUMMARY}"
    #[arg(long, short = 'S')]
    pub summary: bool,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub query: QueryArgs,
}

#[async_trait]
impl RunCommand for MetadataArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;
        let mut client = cfg.connect_restful(false).await?;

        let query = self.query.build_query()?;
        let recycle_time = match self.recycle {
            Some(ref recycle) => Some(time::parse_time(recycle)?),
            None => None,
        };
        let req = GetMetadataRequest {
            id: self.id,
            owner: self.owner.clone(),
            sha256: self.sha256.clone(),
            recycle_before: recycle_time,
            query,
        };

        let resp = client.get_metadatas(req).await?;

        if self.summary {
            for item in resp.items {
                println!("{}. {}", item.id, item.summary);
            }
            return Ok(());
        }

        self.query.display_list(resp)?;

        Ok(())
    }
}
