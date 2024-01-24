use clap::Parser;

/// Clipboard synchronization server.
#[derive(Parser, Debug)]
#[command(author, version = env!("BUILD_VERSION"), about)]
pub struct Config {
    /// The listen address
    #[clap(long, short, default_value = "0.0.0.0:7703")]
    pub addr: String,

    /// The auth password to generate key
    #[clap(long, short)]
    pub password: Option<String>,

    /// Show debug logs
    #[clap(long)]
    pub debug: bool,

    /// Show build info
    #[clap(long)]
    pub build_info: bool,
}
