use std::time::Duration;

use anyhow::Result;
use log::{error, info};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::client::Client;
use crate::clipboard::Clipboard;
use crate::daemon::client::DaemonClient;
use crate::time::current_timestamp;
use crate::types::request::Query;
use crate::types::text::Text;
use crate::types::token::TokenResponse;

pub struct TrayDaemon {
    pub latest_id: Option<u64>,

    pub client: Client,

    pub daemon_port: u16,

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
        info!("Received write request {id}");
        self.refresh_token().await?;
        let text = self.client.read_text(id).await?;
        let text = text.content.unwrap();

        if TcpStream::connect(("127.0.0.1", self.daemon_port))
            .await
            .is_ok()
        {
            info!("Daemon is running, sending text to daemon");
            let daemon_client = DaemonClient::new(self.daemon_port);
            daemon_client.send_data(text.into_bytes()).await?;
            return Ok(());
        }

        info!("Daemon is not running, writing text to clipboard");
        let cb = Clipboard::load()?;
        cb.write_text(text)?;
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

        let mut query = Query::default();
        query.limit = Some(self.limit);

        let texts = self.client.read_texts(query).await?;
        let mut items = Vec::with_capacity(texts.len());
        for text in texts {
            let id = text.id.to_string();
            let text = self.truncate_string(text.content.unwrap());
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

    fn truncate_string(&self, mut s: String) -> String {
        if s.chars().count() <= self.truncate_size {
            return s;
        }

        s.truncate(
            s.char_indices()
                .nth(self.truncate_size)
                .map(|(i, _)| i)
                .unwrap_or(s.len()),
        );
        s.push_str("...");
        s
    }
}
