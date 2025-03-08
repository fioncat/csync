mod blob;
mod metadata;
mod user;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use clap::{Args, Subcommand};
use csync_misc::api::{ListResponse, QueryRequest};
use csync_misc::display::{self, DisplayStyle, TerminalDisplay};
use csync_misc::time;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::RunCommand;

#[derive(Debug, Args)]
pub struct QueryArgs {
    /// The page number to query.
    #[arg(long, short, default_value = "1")]
    pub page: u64,

    /// The number of resources to query per page
    #[arg(long, default_value = "20")]
    pub page_size: u64,

    /// Search with keywords, the field to search is determined by the resource type.
    #[arg(long, short)]
    pub search: Option<String>,

    /// Query resources updated after this date. Format: "unix timestamp, YYYY-MM-DD,
    /// HH:MM:SS, or YYYY-MM-DD HH:MM:SS"
    #[arg(long)]
    pub since: Option<String>,

    /// Query resources updated before this date. Format: "unix timestamp, YYYY-MM-DD,
    /// HH:MM:SS, or YYYY-MM-DD HH:MM:SS"
    #[arg(long)]
    pub until: Option<String>,

    /// When displaying in CSV format, do not show the header row.
    #[arg(long)]
    pub headless: bool,

    /// When displaying in CSV format, manually specify the rows to display.
    #[arg(long)]
    pub csv_titles: Option<String>,

    /// The display style.
    #[arg(short, long, default_value = "table")]
    pub output: DisplayStyle,
}

impl QueryArgs {
    pub fn build_query(&self) -> Result<QueryRequest> {
        if self.page == 0 {
            bail!("page must be greater than 0");
        }

        let offset = (self.page - 1) * self.page_size;
        let limit = self.page_size;

        let update_after = match self.since {
            Some(ref since) => Some(time::parse_time(since).context("parse since")?),
            None => None,
        };

        let update_before = match self.until {
            Some(ref until) => Some(time::parse_time(until).context("parse until")?),
            None => None,
        };

        Ok(QueryRequest {
            offset: Some(offset),
            limit: Some(limit),
            search: self.search.clone(),
            update_after,
            update_before,
        })
    }

    pub fn display_list<T>(&self, list: ListResponse<T>) -> Result<()>
    where
        T: Serialize + DeserializeOwned + TerminalDisplay,
    {
        display::display_list(
            list,
            self.page,
            self.page_size,
            self.output,
            self.headless,
            self.csv_titles.clone(),
        )
    }
}

/// Get commands
#[derive(Args)]
pub struct GetCommand {
    #[command(subcommand)]
    pub command: GetCommands,
}

#[derive(Subcommand)]
pub enum GetCommands {
    Blob(blob::BlobArgs),
    Metadata(metadata::MetadataArgs),
    User(user::UserArgs),
}

#[async_trait]
impl RunCommand for GetCommand {
    async fn run(&self) -> Result<()> {
        match &self.command {
            GetCommands::Blob(args) => args.run().await,
            GetCommands::Metadata(args) => args.run().await,
            GetCommands::User(args) => args.run().await,
        }
    }
}
