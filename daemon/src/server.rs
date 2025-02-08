use std::{fs, io};

use anyhow::{Context, Result};
use csync_misc::config::PathSet;
use csync_misc::daemon::conn::Connection;
use log::{error, info, warn};
use tokio::net::UnixListener;

use crate::sync::send::SyncSender;

pub struct DaemonServer {
    path: String,
    sync_tx: SyncSender,
}

impl DaemonServer {
    pub fn new(ps: &PathSet, sync_tx: SyncSender) -> Self {
        let path = ps.data_path.join("daemon.sock");
        let path = format!("{}", path.display());
        Self { path, sync_tx }
    }

    pub async fn serve(&self) -> Result<()> {
        match fs::metadata(&self.path) {
            Ok(_) => {
                info!("Remove existing daemon socket at: '{}'", self.path);
                fs::remove_file(&self.path).context("remove existing daemon socket")?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("check existing daemon socket"),
        }

        info!("Starting daemon server at: '{}'", self.path);
        let listener = UnixListener::bind(&self.path).context("bind to daemon socket path")?;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    info!("Accept new connection");
                    let mut conn = Connection::new(stream);
                    let mut sync_tx = self.sync_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            let frame = match conn.read_frame().await {
                                Ok(frame) => frame,
                                Err(e) => {
                                    warn!("Read frame error: {:#}, close connection", e);
                                    break;
                                }
                            };
                            match frame {
                                Some(frame) => {
                                    info!(
                                        "Receive {} data from client, send to sync manager",
                                        frame.data.len(),
                                    );
                                    sync_tx.send(frame.data).await;
                                }
                                None => {
                                    info!("Connection closed by peer");
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    error!("Accept connection error: {:#}", e);
                }
            }
        }
    }
}
