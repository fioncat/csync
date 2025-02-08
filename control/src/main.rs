mod cani;
mod cb;
mod config;
mod delete;
mod get;
mod put;
mod read;
mod select;
mod version;
mod whoami;

use std::process;

use anyhow::Result;
use async_trait::async_trait;
use clap::error::ErrorKind as ArgsErrorKind;
use clap::{Args, Parser, Subcommand, ValueEnum};
use csync_misc::time::parse_time;
use csync_misc::types::request::Query;

#[derive(Args, Debug, Clone)]
pub struct QueryArgs {
    /// Offset of the query.
    #[arg(long)]
    pub offset: Option<u64>,

    /// Limit of the query.
    #[arg(long)]
    pub limit: Option<u64>,

    /// Search with keywords, the field to search is determined by the resource type.
    #[arg(long)]
    pub search: Option<String>,

    /// Query resources created after this date. Format: "unix timestamp, YYYY-MM-DD,
    /// HH:MM:SS, or YYYY-MM-DD HH:MM:SS"
    #[arg(long)]
    pub since: Option<String>,

    /// Query resources created before this date. Format: "unix timestamp, YYYY-MM-DD,
    /// HH:MM:SS, or YYYY-MM-DD HH:MM:SS"
    #[arg(long)]
    pub until: Option<String>,

    /// Query resources created by this user
    #[arg(long)]
    pub owner: Option<String>,
}

impl QueryArgs {
    pub fn build_query(&self) -> Result<Query> {
        Ok(Query {
            offset: self.offset,
            limit: self.limit,
            search: self.search.clone(),
            since: match self.since {
                Some(ref since) => Some(parse_time(since)?),
                None => None,
            },
            until: match self.until {
                Some(ref until) => Some(parse_time(until)?),
                None => None,
            },
            owner: self.owner.clone(),
            hash: None,
        })
    }
}

#[async_trait]
pub trait RunCommand {
    async fn run(&self) -> Result<()>;
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ResourceType {
    User,
    Users,

    Role,
    Roles,

    Text,
    Texts,

    Image,
    Images,

    File,
    Files,
}

impl ResourceType {
    pub fn get_name(self) -> &'static str {
        match self {
            ResourceType::User | ResourceType::Users => "users",
            ResourceType::Role | ResourceType::Roles => "roles",
            ResourceType::Text | ResourceType::Texts => "texts",
            ResourceType::Image | ResourceType::Images => "images",
            ResourceType::File | ResourceType::Files => "files",
        }
    }
}

#[derive(Parser)]
#[command(author, about, version = env!("CSYNC_VERSION"))]
pub struct App {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Cani(cani::CaniArgs),
    Cb(cb::CbArgs),
    Config(config::ShowConfigArgs),
    Delete(delete::DeleteArgs),
    Get(get::GetArgs),
    Put(put::PutCommand),
    Read(read::ReadCommand),
    Select(select::SelectArgs),
    Version(version::VersionArgs),
    Whoami(whoami::WhoamiArgs),
}

#[async_trait]
impl RunCommand for App {
    async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Cani(args) => args.run().await,
            Commands::Cb(args) => args.run().await,
            Commands::Config(args) => args.run().await,
            Commands::Delete(args) => args.run().await,
            Commands::Get(args) => args.run().await,
            Commands::Put(args) => args.run().await,
            Commands::Read(args) => args.run().await,
            Commands::Select(args) => args.run().await,
            Commands::Version(args) => args.run().await,
            Commands::Whoami(args) => args.run().await,
        }
    }
}

async fn run_cmd() -> Result<()> {
    let app = match App::try_parse() {
        Ok(app) => app,
        Err(err) => {
            err.use_stderr();
            err.print().expect("write help message to stderr");
            if matches!(
                err.kind(),
                ArgsErrorKind::DisplayHelp
                    | ArgsErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                    | ArgsErrorKind::DisplayVersion
            ) {
                return Ok(());
            }
            process::exit(3);
        }
    };

    app.run().await
}

#[tokio::main]
async fn main() {
    match run_cmd().await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Command error: {e:#}");
            process::exit(1);
        }
    }
}
