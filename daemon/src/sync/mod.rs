pub mod config;
pub mod factory;
pub mod image;
pub mod send;
pub mod text;

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use csync_misc::client::share::ShareClient;
use csync_misc::client::Client;
use csync_misc::clipboard::Clipboard;
use csync_misc::humanize::human_bytes;
use log::{info, warn};
use sha2::{Digest, Sha256};
use tokio::select;
use tokio::sync::mpsc;

#[async_trait]
pub trait ResourceManager {
    async fn read_server_hash(&self, client: &Client) -> Result<Option<String>>;
    async fn read_server(&self, client: &Client) -> Result<Option<Resource>>;

    async fn write_server(&self, client: &Client, data: Vec<u8>) -> Result<()>;

    async fn read_cb(&self, cb: &Clipboard) -> Result<Option<Vec<u8>>>;

    async fn write_cb(&self, cb: &Clipboard, data: Vec<u8>) -> Result<()>;
}

pub struct Resource {
    pub data: Vec<u8>,
    pub hash: String,
}

#[derive(Debug, Clone, Copy)]
enum SyncFlag {
    Push,
    Pull,
    None,
}

pub struct Synchronizer<M: ResourceManager> {
    name: &'static str,

    mgr: M,

    flag: SyncFlag,
    bucket: Option<Resource>,

    server_hash: Option<String>,
    cb_hash: Option<String>,

    share_client: Arc<ShareClient>,

    cb: Clipboard,

    cb_request_rx: mpsc::Receiver<Vec<u8>>,

    server_intv: u64,
    cb_intv: u64,

    server_readonly: bool,
    cb_readonly: bool,

    first_server: bool,
}

impl<M: ResourceManager + Send + 'static> Synchronizer<M> {
    pub fn start(mut self) {
        tokio::spawn(async move {
            self.main_loop().await;
        });
    }

    async fn main_loop(&mut self) {
        info!("[{}] Starting sync main loop", self.name);

        let mut server_intv = tokio::time::interval(Duration::from_millis(self.server_intv));
        let mut cb_intv = tokio::time::interval(Duration::from_millis(self.cb_intv));

        loop {
            select! {
                _ = server_intv.tick() => {
                    if let Err(e) = self.handle_server().await {
                        warn!("[{}] Handle server error: {:#}", self.name, e);
                    }
                }
                _ = cb_intv.tick() => {
                    if let Err(e) = self.handle_cb().await {
                        warn!("[{}] Handle clipboard error: {:#}", self.name, e);
                    }
                }
                Some(data) = self.cb_request_rx.recv() => {
                    if let Err(e) = self.handle_cb_request(data).await {
                        warn!("[{}] Handle clipboard request error: {:#}", self.name, e);
                    }
                }
            }
        }
    }

    async fn handle_server(&mut self) -> Result<()> {
        let client = self.share_client.client().await;

        if let SyncFlag::Push = self.flag {
            let rsc = self.bucket.take().unwrap();
            self.flag = SyncFlag::None;

            let size = human_bytes(rsc.data.len() as u64);
            info!("[{}] Pushing {size} data to server", self.name);
            let start = Instant::now();

            let Resource { data, hash } = rsc;
            self.mgr
                .write_server(client.as_ref(), data)
                .await
                .context("write data to server")?;
            let elapsed = start.elapsed().as_secs_f64();
            info!(
                "[{}] Push done, elapsed: {elapsed:.2}s, hash: {hash}",
                self.name
            );

            self.server_hash = Some(hash);
            return Ok(());
        }

        if self.cb_readonly {
            return Ok(());
        }

        let latest_hash = self.mgr.read_server_hash(client.as_ref()).await?;
        let mut changed = false;
        if latest_hash.is_none() {
            self.server_hash = None;
        } else {
            changed = self.server_hash != latest_hash;
            self.server_hash = latest_hash;
        }
        if self.first_server {
            info!(
                "[{}] First time reading data from server, skip pulling",
                self.name
            );
            self.first_server = false;
            return Ok(());
        }

        if !changed {
            return Ok(());
        }

        info!("[{}] Server data has changed, start pulling", self.name);
        let start = Instant::now();
        let rsc = self.mgr.read_server(client.as_ref()).await?;
        if rsc.is_none() {
            warn!("[{}] Server didn't return any data, skip", self.name);
            return Ok(());
        }
        let rsc = rsc.unwrap();

        self.server_hash = Some(rsc.hash.clone());

        let elapsed = start.elapsed().as_secs_f64();
        info!(
            "[{}] Pull done, elapsed: {elapsed:.2}s, hash: {}",
            self.name, rsc.hash
        );

        self.bucket = Some(rsc);
        self.flag = SyncFlag::Pull;

        Ok(())
    }

    async fn handle_cb(&mut self) -> Result<()> {
        if let SyncFlag::Pull = self.flag {
            let rsc = self.bucket.take().unwrap();
            self.flag = SyncFlag::None;

            info!(
                "[{}] Found dirty pulled data, write it to clipboard",
                self.name
            );

            let Resource { data, hash } = rsc;
            self.mgr.write_cb(&self.cb, data).await?;
            self.cb_hash = Some(hash);
            return Ok(());
        }

        if self.server_readonly {
            return Ok(());
        }

        let data = match self.mgr.read_cb(&self.cb).await? {
            Some(data) => data,
            None => return Ok(()),
        };
        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);
        if let Some(ref cb_hash) = self.cb_hash {
            if cb_hash == &hash {
                return Ok(());
            }
        }

        info!(
            "[{}] Clipboard data has changed, save it to bucket",
            self.name
        );

        self.cb_hash = Some(hash.clone());
        self.bucket = Some(Resource { data, hash });
        self.flag = SyncFlag::Push;

        Ok(())
    }

    async fn handle_cb_request(&mut self, data: Vec<u8>) -> Result<()> {
        let size = data.len();
        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);

        info!(
            "[{}] Writing {} data from user to clipboard, it won't be pushed to server",
            self.name,
            human_bytes(size as u64)
        );

        self.mgr.write_cb(&self.cb, data).await?;
        self.cb_hash = Some(hash);

        Ok(())
    }
}
