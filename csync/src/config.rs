use anyhow::{bail, Result};
use clap::Parser;
use csync_proto::client::{Client, TerminalPassword};
use regex::Regex;

/// Clipboard synchronization client
#[derive(Parser, Debug)]
#[command(author, version = env!("BUILD_VERSION"), about)]
pub struct Config {
    /// The target format is `[<publish>@]<addr>[/<subs>]`.
    /// Both `publish` and `subs` cannot be empty at the same time.
    #[clap(default_value = "default@127.0.0.1")]
    pub target: String,

    /// The port for the csyncd server.
    #[clap(long, short, default_value = "7703")]
    pub port: u32,

    /// Force skipping authentication. If csyncd is configured with
    /// authentication, the program will fail.
    #[clap(long)]
    pub no_auth: bool,

    /// Display synchronization results without outputting the
    /// synchronized clipboard text content to the terminal.
    #[clap(long, short = 'q')]
    pub quiet_content: bool,

    /// Display nothing.
    #[clap(long, short = 'Q')]
    pub quiet_all: bool,

    /// Only output the received content to the terminal without writing
    /// it to the clipboard.
    #[clap(long, short = 'R')]
    pub read_only: bool,

    /// Receive and transmit only text content, ignoring images.
    #[clap(long, short = 'T')]
    pub text_only: bool,

    /// Interval in milliseconds between attempts to fetch data from
    /// the server.
    #[clap(long, short = 'i', default_value = "500")]
    pub pull_interval: u32,

    /// Show build info.
    #[clap(long)]
    pub build_info: bool,
}

pub struct Target {
    pub publish: Option<String>,

    pub host: String,

    pub subs: Option<Vec<String>>,
}

impl Target {
    const TARGET_REGEX: &'static str = r"^([a-zA-Z0-9]*@)*([a-zA-Z0-9\.]*)(/[a-zA-Z0-9,]*)*$";

    pub fn parse<S: AsRef<str>>(s: S) -> Result<Target> {
        let re = Regex::new(Self::TARGET_REGEX).expect("invalid target regex");
        let mut iter = re.captures_iter(s.as_ref());
        let caps = match iter.next() {
            Some(caps) => caps,
            None => bail!("invalid target format '{}'", s.as_ref()),
        };

        if caps.get(0).is_none() {
            bail!("invalid target '{}', did not match regex", s.as_ref());
        }

        let mut publish = None;
        if let Some(publish_name) = caps.get(1) {
            let name = publish_name
                .as_str()
                .strip_suffix('@')
                .unwrap_or(publish_name.as_str());
            publish = Some(name.to_string());
        }

        let host = match caps.get(2) {
            Some(host) => host.as_str().to_string(),
            None => bail!("invalid target '{}', missing host", s.as_ref()),
        };

        let mut subs = None;
        if let Some(sub_names) = caps.get(3) {
            let sub_names = sub_names
                .as_str()
                .strip_prefix('/')
                .unwrap_or(sub_names.as_str());
            subs = Some(
                sub_names
                    .split(',')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
        }

        if publish.is_none() && subs.is_none() {
            bail!(
                "invalid target '{}', you must provide publish or subs in target",
                s.as_ref()
            );
        }

        Ok(Target {
            publish,
            host,
            subs,
        })
    }

    pub async fn build_client(self, cfg: &Config) -> Result<Client<TerminalPassword>> {
        let addr = format!("{}:{}", self.host, cfg.port);
        Client::dial(
            addr,
            self.publish,
            self.subs,
            TerminalPassword::new(cfg.no_auth),
        )
        .await
    }
}
