mod clipboard;
mod config;
#[cfg(target_os = "linux")]
mod daemonize;
mod remote;
mod server;
mod tray;

use std::process;

use anyhow::Result;
use clap::Parser;
#[cfg(target_os = "linux")]
use clap::ValueEnum;
use config::DaemonConfig;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::{ConfigArgs, PathSet};
use csync_misc::display;
use csync_misc::filelock::FileLock;
use log::info;

#[derive(Parser, Debug)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
struct DaemonArgs {
    /// Daemon action.
    #[cfg(target_os = "linux")]
    pub action: Option<DaemonizeAction>,

    /// Print daemon and client configuration data (JSON) and exit.
    #[arg(long)]
    pub print_config: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DaemonizeAction {
    Start,
    Restart,
    Status,
    Stop,
    Out,
    Logs,
}

#[cfg(target_os = "linux")]
fn handle_daemonize(args: &DaemonArgs, ps: &PathSet) -> Result<()> {
    if let Some(action) = args.action {
        let daemon = daemonize::Daemonize::new(ps)?;
        if !daemon.handle(action)? {
            process::exit(0);
        }
    }

    Ok(())
}

fn blocking_main(args: DaemonArgs) -> Result<()> {
    let ps = args.config.build_path_set()?;
    let client_cfg: ClientConfig = args.config.load_from_path_set("client", &ps)?;
    let daemon_cfg: DaemonConfig = args.config.load_from_path_set("daemon", &ps)?;

    if args.print_config {
        display::pretty_json(client_cfg)?;
        display::pretty_json(daemon_cfg)?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    handle_daemonize(&args, &ps)?;

    tokio_main(ps, client_cfg, daemon_cfg);
}

fn tokio_main(ps: PathSet, client_cfg: ClientConfig, daemon_cfg: DaemonConfig) -> ! {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create tokio runtime: {e:#}");
            process::exit(2);
        }
    };
    rt.block_on(async move {
        match run(ps, client_cfg, daemon_cfg).await {
            Ok(_) => {
                info!("Daemon exited successfully");
            }
            Err(e) => {
                eprintln!("Daemon error: {e:#}");
                process::exit(1);
            }
        }
    });
    process::exit(0);
}

async fn run(ps: PathSet, client_cfg: ClientConfig, daemon_cfg: DaemonConfig) -> Result<()> {
    client_cfg.logs.init("daemon")?;

    let lock_path = ps.data_dir.join("daemon.lock");
    let lock = FileLock::acquire(lock_path)?;

    let remote = daemon_cfg.build_remote(&client_cfg).await?;
    let copy_tx = daemon_cfg.start_clipboard(remote.clone())?;

    let server = daemon_cfg.build_server(&client_cfg, copy_tx.clone());
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Failed to start daemon server: {e:#}");
            process::exit(1);
        }
    });

    let tray = daemon_cfg.build_tray(remote.clone(), ps.clone(), copy_tx);
    tray.run()?;
    drop(lock);

    Ok(())
}

fn main() {
    let args = DaemonArgs::parse();

    match blocking_main(args) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e:#}");
            process::exit(1);
        }
    }
}
