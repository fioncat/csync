mod now;
mod server;
mod sync;
mod tray;

use std::process;

use anyhow::Result;
use clap::Parser;
use csync_misc::client::config::ClientConfig;
use csync_misc::config::CommonConfig;
use csync_misc::filelock::GlobalLock;
use csync_misc::types::cmd::{ConfigArgs, LogArgs};
use log::{error, info};
use server::DaemonServer;
use sync::config::SyncConfig;
use sync::factory::SyncFactory;
use sync::send::SyncSender;
use tray::factory::TrayFactory;
use tray::ui::build_and_run_tray_ui;

#[derive(Parser, Debug)]
#[command(author, version = env!("CSYNC_VERSION"), about)]
struct DaemonArgs {
    /// Do not start the system tray
    #[arg(short, long)]
    pub no_tray: bool,

    /// Tray option, maximum number of history entries to display
    #[arg(short, long, default_value = "20")]
    pub limit: u64,

    /// Tray option, text entries longer than this size will be truncated
    #[arg(short, long, default_value = "80")]
    pub truncate_size: usize,

    #[command(flatten)]
    pub config: ConfigArgs,

    #[command(flatten)]
    pub log: LogArgs,
}

async fn run(args: DaemonArgs) -> Result<()> {
    args.log.init()?;
    let ps = args.config.build_path_set()?;

    let lock_path = ps.data_path.join("daemon.lock");
    let lock = GlobalLock::acquire(lock_path)?;

    let client_cfg: ClientConfig = ps.load_config("client", ClientConfig::default)?;
    let sync_cfg: SyncConfig = ps.load_config("sync", SyncConfig::default)?;

    let factory = SyncFactory::new(client_cfg.clone(), sync_cfg).await?;

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
    if args.no_tray {
        return srv.serve().await;
    }

    tokio::spawn(async move {
        if let Err(e) = srv.serve().await {
            error!("Start daemon server error: {:#}", e);
            process::exit(1);
        }
    });

    let tray_factory = TrayFactory::new(client_cfg);
    let (mut tray_daemon, menu_rx, write_tx) = tray_factory
        .build_tray_daemon(args.limit, args.truncate_size, sync_tx)
        .await?;

    let default_menu = tray_daemon.build_menu().await?;

    tokio::spawn(async move {
        tray_daemon.run().await;
    });

    build_and_run_tray_ui(default_menu, menu_rx, write_tx).await?;
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
