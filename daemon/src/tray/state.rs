use std::collections::HashMap;
use std::time::Duration;

use csync_misc::api::metadata::{Event, EventType, Metadata};
use log::{debug, error, info};
use tokio::{select, sync::mpsc};

use crate::remote::Remote;

#[derive(Debug, Clone, Default)]
pub struct TrayState {
    pub items: Vec<Metadata>,
    pub total: u64,
    pub events_error: bool,
    pub fetch_error: bool,
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
        let mut events_sub = self.remote.subscribe().await;
        let mut refresh_intv = tokio::time::interval(Duration::from_secs(self.refresh_secs));

        loop {
            select! {
                event = events_sub.events.recv() => {
                    self.handle_event(event.unwrap()).await;
                }

                state = events_sub.states.recv() => {
                    self.handle_event_state(state.unwrap()).await;
                }

                _ = refresh_intv.tick() => {
                    self.handle_refresh().await;
                }
            }
        }
    }

    async fn handle_event(&mut self, event: Event) {
        debug!("Tray State: handle event: {:#?}", event);
        self.data.events_error = false;
        if matches!(event.event_type, EventType::Put) {
            debug!("Tray State: new item added, need refresh state");
            self.updated = true;
            return;
        }

        let updated: HashMap<u64, Metadata> = event
            .items
            .into_iter()
            .map(|item| (item.id, item))
            .collect();
        for item in self.data.items.iter() {
            if updated.contains_key(&item.id) {
                debug!(
                    "Tray State: item {} updated or deleted, need refresh state",
                    item.id
                );
                self.updated = true;
                return;
            }
        }
    }

    async fn handle_event_state(&mut self, state: bool) {
        debug!("Tray State: receive event error, need refresh state");

        self.data.events_error = !state;
        self.updated = true;
    }

    async fn handle_refresh(&mut self) {
        if self.data.fetch_error {
            self.reset_state(false).await;
            if self.data.fetch_error {
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
                self.data.fetch_error = false;
            }
            Err(e) => {
                if with_logs {
                    error!("Tray State: fetch data from server error: {e:#}");
                }
                self.data.fetch_error = true;
            }
        }
    }
}
