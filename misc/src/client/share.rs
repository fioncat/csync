use anyhow::{bail, Result};
use chrono::Local;
use log::{info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;

use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, Interval};

use crate::client::factory::ClientFactory;

use super::config::ClientConfig;
use super::Client;

pub async fn build_share_client(cfg: ClientConfig) -> Result<Arc<ShareClient>> {
    let user = cfg.user.clone();
    let password = cfg.password.clone();

    let factory = ClientFactory::new(cfg);
    let client = factory.build_client().await?;
    let req_tx = ShareManager::start(client, user, password).await?;

    let share_client = ShareClient { req_tx };
    Ok(Arc::new(share_client))
}

pub struct ShareClient {
    req_tx: mpsc::Sender<oneshot::Sender<Arc<Client>>>,
}

impl ShareClient {
    pub async fn client(&self) -> Arc<Client> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.req_tx.send(resp_tx).await.unwrap();
        resp_rx.await.unwrap()
    }
}

struct ShareManager {
    client: Arc<Client>,
    user: String,
    password: String,
    req_rx: mpsc::Receiver<oneshot::Sender<Arc<Client>>>,
    refresh_intv: Interval,
}

impl ShareManager {
    async fn start(
        client: Client,
        user: String,
        password: String,
    ) -> Result<mpsc::Sender<oneshot::Sender<Arc<Client>>>> {
        let (req_tx, req_rx) = mpsc::channel(500);
        let mut mgr = Self {
            client: Arc::new(client),
            user,
            password,
            req_rx,
            refresh_intv: interval(Duration::from_secs(1)),
        };

        mgr.refresh_token_inner().await?;

        tokio::spawn(async move {
            mgr.main_loop().await;
        });

        Ok(req_tx)
    }

    async fn main_loop(mut self) {
        loop {
            select! {
                _ = self.refresh_intv.tick() => {
                    self.refresh_token().await;
                }
                Some(resp) = self.req_rx.recv() => {
                    let client = self.client.clone();
                    resp.send(client).unwrap();
                }
            }
        }
    }

    async fn refresh_token(&mut self) {
        if let Err(e) = self.refresh_token_inner().await {
            // When refreshing failed, we need to retry after a short time
            self.refresh_intv.reset_after(Duration::from_secs(5));
            warn!("Failed to refresh share client token: {:#}", e);
        }
    }

    async fn refresh_token_inner(&mut self) -> Result<()> {
        info!("Refreshing share client token");

        let resp = self.client.login(&self.user, &self.password).await?;
        let new_client = self.client.derive(resp.token);
        self.client = Arc::new(new_client);

        let expire = resp.expire_in - Client::MAX_TIME_DELTA_WITH_SERVER;
        let now = Local::now().timestamp() as usize;
        if now >= expire {
            bail!("Token is immediately expired after refresh, the server or client time may be incorrect");
        }
        let delta = expire - now;
        self.refresh_intv
            .reset_after(Duration::from_secs(delta as u64));
        info!("The next time to refresh the token is after {delta}s");

        Ok(())
    }
}
