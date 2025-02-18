mod config;
mod daemonize;
mod server;
mod sync;
mod tray;

use std::process;

use anyhow::Result;
use clap::Parser;
use config::DaemonConfig;
use csync_misc::client::config::ClientConfig;
use csync_misc::client::share::build_share_client;
use csync_misc::config::{CommonConfig, ConfigArgs, PathSet};
use csync_misc::display::display_json;
use csync_misc::filelock::GlobalLock;
use log::{error, info};
use serde::Serialize;
use server::DaemonServer;
use sync::factory::SyncFactory;
use sync::send::SyncSender;
use tray::factory::TrayFactory;
use tray::ui::run_tray_ui;

#[derive(Parser, Debug)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
struct DaemonArgs {
    /// Print daemon and client configuration data (JSON) and exit.
    #[arg(long)]
    pub print_config: bool,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[derive(Debug, Serialize)]
struct ConfigSet {
    client: ClientConfig,
    daemon: DaemonConfig,
}

fn blocking_main(args: DaemonArgs) -> Result<()> {
    let ps = args.config.build_path_set()?;
    let client_cfg: ClientConfig = ps.load_config("client", ClientConfig::default)?;
    let daemon_cfg = ps.load_config("daemon", DaemonConfig::default)?;
    if args.print_config {
        return display_json(&ConfigSet {
            client: client_cfg,
            daemon: daemon_cfg,
        });
    }

    tokio_main(ps, client_cfg, daemon_cfg);
}

fn tokio_main(ps: PathSet, client_cfg: ClientConfig, daemon_cfg: DaemonConfig) -> ! {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to create tokio runtime: {:#}", e);
            process::exit(2);
        }
    };
    rt.block_on(async move {
        match run(ps, client_cfg, daemon_cfg).await {
            Ok(_) => {
                info!("Daemon exited successfully");
            }
            Err(e) => {
                error!("Daemon error: {:#}", e);
                process::exit(1);
            }
        }
    });
    process::exit(0);
}

async fn run(ps: PathSet, client_cfg: ClientConfig, daemon_cfg: DaemonConfig) -> Result<()> {
    ps.init_logger("daemon", &daemon_cfg.log)?;

    let lock_path = ps.data_path.join("daemon.lock");
    let lock = GlobalLock::acquire(lock_path)?;

    let share_client = build_share_client(client_cfg).await?;

    let factory = SyncFactory::new(daemon_cfg.sync).await?;

    let mut sync_tx = SyncSender::default();
    if let Some((sync, tx)) = factory.build_text_sync(share_client.clone()) {
        sync_tx.text_tx = Some(tx);
        sync.start();
    }

    if let Some((sync, tx)) = factory.build_image_sync(share_client.clone()) {
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

    let tray_factory = TrayFactory::new(daemon_cfg.tray);
    let api = tray_factory.build_tray_api_handler(share_client, ps, sync_tx);
    let default_menu = api.build_menu().await?;

    run_tray_ui(api, default_menu)?;
    drop(lock);

    Ok(())
}

fn main() {
    let args = DaemonArgs::parse();

    match blocking_main(args) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {:#}", e);
            process::exit(1);
        }
    }
}
