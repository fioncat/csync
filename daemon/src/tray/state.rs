use std::time::Duration;

use csync_misc::api::metadata::Metadata;
use log::{debug, error, info};
use tokio::{select, sync::mpsc};

use crate::remote::Remote;

#[derive(Debug, Clone, Default)]
pub struct TrayState {
    pub items: Vec<Metadata>,
    pub total: u64,
    pub revision: u64,
    pub server_error: bool,
}

impl TrayState {
    pub fn start(remote: Remote, limit: u64, refresh_secs: u64) -> mpsc::Receiver<Self> {
        let (state_tx, state_rx) = mpsc::channel(100);

        let handler = TrayStateHandler {
            updated: true,
            refresh_secs,
            data: TrayState::default(),
            remote,
            limit,
            state_tx,
        };
        tokio::spawn(async move {
            handler.main_loop().await;
        });

        state_rx
    }
}

struct TrayStateHandler {
    updated: bool,

    refresh_secs: u64,
    data: TrayState,

    remote: Remote,

    limit: u64,

    state_tx: mpsc::Sender<TrayState>,
}

impl TrayStateHandler {
    async fn main_loop(mut self) {
        info!("Start tray state main loop");
        let mut refresh_intv = tokio::time::interval(Duration::from_secs(self.refresh_secs));
        let mut rev_intv = tokio::time::interval(Duration::from_secs(1));

        loop {
            select! {
                _ = rev_intv.tick() => {
                    self.handle_revision().await;
                }

                _ = refresh_intv.tick() => {
                    self.handle_refresh().await;
                }
            }
        }
    }

    async fn handle_revision(&mut self) {
        let (rev, ok) = self.remote.get_revision().await;
        if !ok {
            self.data.server_error = true;
            return;
        }

        let rev = match rev {
            Some(rev) => match rev.rev {
                Some(rev) => rev,
                None => return,
            },
            None => return,
        };

        if rev == self.data.revision {
            return;
        }
        info!("Tray State: new revision: {}, need refresh state", rev);
        self.data.revision = rev;
        self.updated = true;
    }

    async fn handle_refresh(&mut self) {
        if self.data.server_error {
            self.reset_state(false).await;
            if self.data.server_error {
                return;
            }

            debug!("Tray State: fetch data error recovered");
            let state = self.data.clone();
            self.state_tx.send(state).await.unwrap();
            return;
        }

        if !self.updated {
            return;
        }

        self.reset_state(true).await;
        let state = self.data.clone();
        self.state_tx.send(state).await.unwrap();
        self.updated = false;
    }

    async fn reset_state(&mut self, with_logs: bool) {
        match self.remote.get_metadatas(self.limit).await {
            Ok(list) => {
                self.data.items = list.items;
                self.data.total = list.total;
                self.data.server_error = false;
            }
            Err(e) => {
                if with_logs {
                    error!("Tray State: fetch data from server error: {e:#}");
                }
                self.data.server_error = true;
            }
        }
    }
}
