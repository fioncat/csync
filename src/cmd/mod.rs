mod cani;
mod cb;
mod config;
mod daemon;
mod delete;
mod get;
mod put;
mod read;
mod server;
#[cfg(feature = "tray")]
mod tray;
mod version;
mod whoami;

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::config::PathSet;
use crate::logs;
use crate::time::parse_time;
use crate::types::request::Query;
use crate::types::server::Server;

#[derive(Parser)]
#[command(author, about, version = env!("CSYNC_VERSION"))]
pub struct App {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Config(config::ShowConfigArgs),
    Delete(delete::DeleteArgs),
    Get(get::GetArgs),
    Put(put::PutCommand),
    Read(read::ReadCommand),
    Cb(cb::CbArgs),
    Whoami(whoami::WhoamiArgs),
    Cani(cani::CaniArgs),
    Version(version::VersionArgs),
    #[cfg(feature = "tray")]
    Tray(tray::TrayArgs),

    Server(server::ServerArgs),
    Daemon(daemon::DaemonArgs),
}

#[async_trait]
impl RunCommand for App {
    async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Config(args) => args.run().await,
            Commands::Delete(args) => args.run().await,
            Commands::Get(args) => args.run().await,
            Commands::Put(args) => args.run().await,
            Commands::Read(args) => args.run().await,
            Commands::Cb(args) => args.run().await,
            Commands::Whoami(args) => args.run().await,
            Commands::Cani(args) => args.run().await,
            Commands::Version(args) => args.run().await,
            #[cfg(feature = "tray")]
            Commands::Tray(args) => args.run().await,

            Commands::Server(_) => unreachable!(),
            Commands::Daemon(_) => unreachable!(),
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    /// Configuration directory path, all configuration files will be read from this path.
    /// The directory will be automatically created if it does not exist. Note that certain
    /// behaviors of csync will generate specific files in this directory (if you have not
    /// provided them)
    #[arg(long)]
    pub config_path: Option<String>,

    /// Data directory path. Persistent data generated by the process will be stored in this
    /// directory. The directory will be automatically created if it does not exist
    #[arg(long)]
    pub data_path: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct LogArgs {
    /// Log level.
    #[arg(long, default_value = "info")]
    pub log_level: String,
}

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

impl ConfigArgs {
    pub fn build_path_set(&self) -> Result<PathSet> {
        PathSet::new(
            self.config_path.clone().map(PathBuf::from),
            self.data_path.clone().map(PathBuf::from),
        )
    }
}

impl LogArgs {
    pub fn init(&self) -> Result<()> {
        logs::init(&self.log_level)
    }
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

#[async_trait]
pub trait ServerCommand {
    async fn build_server(&self) -> Result<Server>;
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
