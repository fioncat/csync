use std::fs;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, ffi::OsString};

use anyhow::bail;
use anyhow::{Context, Result};
use clap::Parser;

use std::net::SocketAddr;

/// Sync clipboard between different machines via network.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arg {
    /// TCP bind address.
    #[arg(short, long, default_value = "0.0.0.0:9790")]
    pub bind: String,

    /// Target addresses to sync clipboard, split with comma.
    #[arg(short, long, default_value = "")]
    pub target: String,

    /// Interval (ms) to listen clipboard, must be in the range [50, 3000].
    #[arg(short, long, default_value = "300")]
    pub interval: u64,

    /// The directory to write sync file.
    #[arg(short, long, default_value = "")]
    pub dir: String,

    /// The file to send.
    #[arg(short, long)]
    pub file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,

    pub targets: Vec<SocketAddr>,

    pub interval: u64,

    pub dir: PathBuf,
}

impl Arg {
    pub fn normalize(&mut self) -> Result<Config> {
        if let Some(s) = env::var_os("CSYNC_CONFIG_BIND") {
            self.bind = parse_osstr(s)?;
        }
        if self.bind.is_empty() {
            bail!("Bind address could not be empty");
        }
        let bind: SocketAddr = self
            .bind
            .parse()
            .with_context(|| format!(r#"Invalid bind address "{}""#, self.bind))?;

        if let Some(s) = env::var_os("CSYNC_CONFIG_TARGET") {
            self.target = parse_osstr(s)?;
        }
        if self.target.is_empty() {
            bail!("Invalid usage, the target could not be empty, please use `--target` or env `CSYNC_CONFIG_TARGET`");
        }
        let endpoints: Vec<_> = self.target.split(",").collect();
        let mut targets: Vec<SocketAddr> = Vec::with_capacity(endpoints.len());
        for ep in endpoints {
            let addr: SocketAddr = ep
                .parse()
                .with_context(|| format!(r#"Could not parse target address "{}""#, ep))?;
            targets.push(addr);
        }

        if let Some(s) = env::var_os("CSYNC_CONFIG_INTERVAL") {
            let interval = parse_osstr(s)?;
            let interval: u64 = interval.parse().context("Could not parse interval")?;
            self.interval = interval;
        }
        if self.interval < 50 || self.interval > 3000 {
            bail!(
                "Invalid interval {}, It must be in the range [50,3000]",
                self.interval
            );
        }

        if let Some(s) = env::var_os("CSYNC_CONFIG_DIR") {
            self.dir = parse_osstr(s)?;
        }
        let dir = if self.dir.is_empty() {
            dirs::data_local_dir()
                .context("Could not get data dir, please specify dir manually")?
                .join("csync")
        } else {
            PathBuf::from_str(&self.dir).context("Could not parse dir string")?
        };
        match fs::read_dir(&dir) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(&dir)
                    .with_context(|| format!(r#"Could not create data dir "{}""#, dir.display()))?;
            }
            Err(err) => {
                Err(err)
                    .with_context(|| format!(r#"Could not read data dir "{}""#, dir.display()))?;
            }
            Ok(_) => {}
        }

        Ok(Config {
            bind,
            targets,
            interval: self.interval,
            dir,
        })
    }
}

pub fn parse_osstr(s: OsString) -> Result<String> {
    match s.to_str() {
        Some(s) => Ok(s.to_string()),
        None => bail!("Parse string failed, please check your config"),
    }
}
