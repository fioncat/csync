mod config;
mod now;
mod server;
mod sync;
mod tray;

use std::process;

use anyhow::Result;
use clap::Parser;
use config::DaemonConfig;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::CommonConfig;
use csync_misc::display::display_json;
use csync_misc::filelock::GlobalLock;
use csync_misc::types::cmd::{ConfigArgs, LogArgs};
use log::{error, info};
use serde::Serialize;
use server::DaemonServer;
use sync::factory::SyncFactory;
use sync::send::SyncSender;
use tray::factory::TrayFactory;
use tray::ui::build_and_run_tray_ui;

#[derive(Parser, Debug)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
struct DaemonArgs {
    /// Print daemon and client configuration data (JSON) and exit.
    #[arg(long)]
    pub print_config: bool,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub log: LogArgs,
}

#[derive(Debug, Serialize)]
struct ConfigSet {
    client: ClientConfig,
    daemon: DaemonConfig,
}

async fn run(args: DaemonArgs) -> Result<()> {
    args.log.init()?;
    let ps = args.config.build_path_set()?;
    let client_cfg: ClientConfig = ps.load_config("client", ClientConfig::default)?;
    let daemon_cfg = ps.load_config("daemon", DaemonConfig::default)?;
    if args.print_config {
        display_json(&ConfigSet {
            client: client_cfg,
            daemon: daemon_cfg,
        })?;
        process::exit(0);
    }

    let lock_path = ps.data_path.join("daemon.lock");
    let lock = GlobalLock::acquire(lock_path)?;

    let allow_save = daemon_cfg.tray.allow_save;

    let factory = SyncFactory::new(client_cfg.clone(), daemon_cfg.sync).await?;

    let mut sync_tx = SyncSender::default();
    if let Some((sync, tx)) = factory.build_text_sync() {
        sync_tx.text_tx = Some(tx);
        sync.start();
    }

    if let Some((sync, tx)) = factory.build_image_sync() {
        sync_tx.image_tx = Some(tx);
        sync.start();
    }

    let srv = DaemonServer::new(&ps, sync_tx.clone());
    if !daemon_cfg.tray.enable {
        return srv.serve().await;
    }

    tokio::spawn(async move {
        if let Err(e) = srv.serve().await {
            error!("Start daemon server error: {:#}", e);
            process::exit(1);
        }
    });

    let tray_factory = TrayFactory::new(daemon_cfg.tray, client_cfg);
    let (mut tray_daemon, menu_rx, write_tx) = tray_factory.build_tray_daemon(sync_tx).await?;

    let default_menu = tray_daemon.build_menu().await?;

    tokio::spawn(async move {
        tray_daemon.run().await;
    });

    build_and_run_tray_ui(default_menu, menu_rx, write_tx, allow_save).await?;
    drop(lock);

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = DaemonArgs::parse();

    match run(args).await {
        Ok(_) => info!("Daemon exited successfully"),
        Err(e) => {
            error!("Failed to run daemon: {:#}", e);
            process::exit(1);
        }
    }
}
