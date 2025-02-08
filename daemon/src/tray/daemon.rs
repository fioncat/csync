use std::time::Duration;

use anyhow::Result;
use csync_misc::client::Client;
use csync_misc::types::request::Query;
use csync_misc::types::text::{truncate_text, Text};
use csync_misc::types::token::TokenResponse;
use log::{error, info};
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::now::current_timestamp;
use crate::sync::send::SyncSender;

pub struct TrayDaemon {
    pub latest_id: Option<u64>,

    pub client: Client,

    pub sync_tx: SyncSender,

    pub token: Option<TokenResponse>,
    pub user: String,
    pub password: String,

    pub menu_tx: mpsc::Sender<Vec<(String, String)>>,
    pub write_rx: mpsc::Receiver<u64>,

    pub limit: u64,
    pub truncate_size: usize,
}

impl TrayDaemon {
    const REFRESH_MENU_INTERVAL: Duration = Duration::from_secs(1);

    pub async fn run(mut self) {
        info!("Starting tray daemon");
        let mut intv = interval(Self::REFRESH_MENU_INTERVAL);
        loop {
            select! {
                _ = intv.tick() => {
                    let need_refresh = match self.need_refresh_menu().await {
                        Ok(need_refresh) => need_refresh,
                        Err(e) => {
                            error!("Check need refresh menu error: {:#}", e);
                            continue;
                        }
                    };
                    if need_refresh {
                        let menu = match self.build_menu().await {
                            Ok(menu) => menu,
                            Err(e) => {
                                error!("Build menu error: {:#}", e);
                                continue;
                            }
                        };
                        self.menu_tx.send(menu).await.unwrap();
                    }
                },

                Some(id) = self.write_rx.recv() => {
                    if let Err(e) = self.write_text(id).await {
                        error!("Write text error: {:#}", e);
                    }
                }
            }
        }
    }

    async fn write_text(&mut self, id: u64) -> Result<()> {
        info!("Received write request {id}, sending to daemon server");
        self.refresh_token().await?;
        let text = self.client.read_text(id).await?;
        let text = text.content.unwrap();

        self.sync_tx.send(text.into_bytes()).await;
        Ok(())
    }

    async fn need_refresh_menu(&mut self) -> Result<bool> {
        self.refresh_token().await?;

        let latest_text: Text = self
            .client
            .get_resource("texts", String::from("latest"))
            .await?;
        let latest_id = Some(latest_text.id);

        if self.latest_id != latest_id {
            info!(
                "Latest text id changed to {}, need refresh menu",
                latest_text.id
            );
            self.latest_id = latest_id;
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn build_menu(&mut self) -> Result<Vec<(String, String)>> {
        self.refresh_token().await?;

        let query = Query {
            limit: Some(self.limit),
            ..Default::default()
        };

        let texts = self.client.read_texts(query).await?;
        let mut items = Vec::with_capacity(texts.len());
        for text in texts {
            let id = text.id.to_string();
            let text = truncate_text(text.content.unwrap(), self.truncate_size);
            let text = text.replace("\n", "\\n");
            items.push((id, text));
        }

        Ok(items)
    }

    async fn refresh_token(&mut self) -> Result<()> {
        let mut need_flush = true;
        if let Some(ref token) = self.token {
            let now = current_timestamp() as usize;
            if now < token.expire_in {
                need_flush = false;
            }
        }

        if !need_flush {
            return Ok(());
        }

        info!("Refreshing client token");
        let mut resp = self.client.login(&self.user, &self.password).await?;
        self.client.set_token(resp.token.clone());

        resp.expire_in -= Client::MAX_TIME_DELTA_WITH_SERVER;
        info!("Token refreshed, expire_in: {}", resp.expire_in);

        self.token = Some(resp);
        Ok(())
    }
}
