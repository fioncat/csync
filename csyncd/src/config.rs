use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version = env!("BUILD_VERSION"), about)]
pub struct Config {
    #[clap(long, short, default_value = "0.0.0.0:7703")]
    pub addr: String,

    #[clap(long, short)]
    pub password: Option<String>,

    #[clap(long)]
    pub debug: bool,

    #[clap(long)]
    pub build_info: bool,
}
