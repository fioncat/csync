use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use csync_misc::api::blob::{Blob, PatchBlobRequest};
use csync_misc::api::metadata::{GetMetadataRequest, Metadata, ServerState};
use csync_misc::api::{ListResponse, QueryRequest};
use csync_misc::client::restful::RestfulClient;
use log::{debug, error, info};
use tokio::select;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub struct Remote {
    get_state_tx: mpsc::Sender<GetStateRequest>,
    get_metadatas_tx: mpsc::Sender<GetMetadatasRequest>,
    put_blob_tx: mpsc::Sender<PutBlobRequest>,
    get_blob_tx: mpsc::Sender<GetBlobRequest>,
    delete_blob_tx: mpsc::Sender<DeleteBlobRequest>,
    pin_blob_tx: mpsc::Sender<PinBlobRequest>,
}

impl Remote {
    pub fn start(client: RestfulClient, cache_seconds: u64) -> Self {
        let (get_state_tx, get_state_rx) = mpsc::channel(500);
        let (get_metadatas_tx, get_metadatas_rx) = mpsc::channel(500);
        let (put_blob_tx, put_blob_rx) = mpsc::channel(500);
        let (get_blob_tx, get_blob_rx) = mpsc::channel(500);
        let (delete_blob_tx, delete_blob_rx) = mpsc::channel(500);
        let (pin_blob_tx, pin_blob_rx) = mpsc::channel(500);

        let handler = RemoteHandler {
            client,
            state: None,
            state_error: false,
            blobs_cache: HashMap::new(),
            cache_seconds,
            get_state_rx,
            get_metadatas_rx,
            put_blob_rx,
            get_blob_rx,
            delete_blob_rx,
            pin_blob_rx,
        };
        tokio::spawn(async move {
            handler.main_loop().await;
        });

        Self {
            get_state_tx,
            get_metadatas_tx,
            put_blob_tx,
            get_blob_tx,
            delete_blob_tx,
            pin_blob_tx,
        }
    }

    pub async fn get_state(&self) -> (Option<Arc<ServerState>>, bool) {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.get_state_tx
            .send(GetStateRequest { resp: resp_tx })
            .await
            .unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn get_metadatas(&self, limit: u64) -> Result<ListResponse<Metadata>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.get_metadatas_tx
            .send(GetMetadatasRequest {
                limit,
                resp: resp_tx,
            })
            .await
            .unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn put_blob(&self, blob: Blob) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.put_blob_tx
            .send(PutBlobRequest {
                blob,
                resp: resp_tx,
            })
            .await
            .unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn get_blob(&self, id: u64) -> Result<Blob> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.get_blob_tx
            .send(GetBlobRequest { id, resp: resp_tx })
            .await
            .unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn delete_blob(&self, id: u64) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.delete_blob_tx
            .send(DeleteBlobRequest { id, resp: resp_tx })
            .await
            .unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn pin_blob(&self, id: u64, pin: bool) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.pin_blob_tx
            .send(PinBlobRequest {
                id,
                pin,
                resp: resp_tx,
            })
            .await
            .unwrap();
        resp_rx.await.unwrap()
    }
}

struct GetStateRequest {
    resp: oneshot::Sender<(Option<Arc<ServerState>>, bool)>,
}

struct GetMetadatasRequest {
    limit: u64,
    resp: oneshot::Sender<Result<ListResponse<Metadata>>>,
}

struct PutBlobRequest {
    blob: Blob,
    resp: oneshot::Sender<Result<()>>,
}

struct GetBlobRequest {
    id: u64,
    resp: oneshot::Sender<Result<Blob>>,
}

struct DeleteBlobRequest {
    id: u64,
    resp: oneshot::Sender<Result<()>>,
}

struct PinBlobRequest {
    id: u64,
    pin: bool,
    resp: oneshot::Sender<Result<()>>,
}

struct CacheBlob {
    blob: Blob,
    expire: u64,
}

struct RemoteHandler {
    client: RestfulClient,
    state: Option<Arc<ServerState>>,
    state_error: bool,

    blobs_cache: HashMap<u64, CacheBlob>,
    cache_seconds: u64,

    get_state_rx: mpsc::Receiver<GetStateRequest>,
    get_metadatas_rx: mpsc::Receiver<GetMetadatasRequest>,
    put_blob_rx: mpsc::Receiver<PutBlobRequest>,
    get_blob_rx: mpsc::Receiver<GetBlobRequest>,
    delete_blob_rx: mpsc::Receiver<DeleteBlobRequest>,
    pin_blob_rx: mpsc::Receiver<PinBlobRequest>,
}

impl RemoteHandler {
    async fn main_loop(mut self) {
        let mut recycle_cache_intv = tokio::time::interval(Duration::from_secs(self.cache_seconds));
        let mut refresh_state_intv = tokio::time::interval(Duration::from_secs(1));
        info!("Begin to handle remote requests");

        loop {
            select! {
                _ = refresh_state_intv.tick() => {
                    self.refresh_state().await;
                },

                Some(req) = self.get_state_rx.recv() => {
                    let result = self.handle_get_state().await;
                    req.resp.send(result).unwrap();
                }

                Some(req) = self.get_metadatas_rx.recv() => {
                    let result = self.handle_get_metadatas(req.limit).await;
                    req.resp.send(result).unwrap();
                }

                Some(req) = self.put_blob_rx.recv() => {
                    let result = self.handle_put_blob(req.blob).await;
                    req.resp.send(result).unwrap();
                }

                Some(req) = self.get_blob_rx.recv() => {
                    let result = self.handle_get_blob(req.id).await;
                    req.resp.send(result).unwrap();
                }

                Some(req) = self.delete_blob_rx.recv() => {
                    let result = self.handle_delete_blob(req.id).await;
                    req.resp.send(result).unwrap();
                }

                Some(req) = self.pin_blob_rx.recv() => {
                    let result = self.handle_pin_blob(req.id, req.pin).await;
                    req.resp.send(result).unwrap();
                }

                _ = recycle_cache_intv.tick() => {
                    self.handle_recycle_cache();
                }
            }
        }
    }

    async fn refresh_state(&mut self) {
        let latest_state = match self.client.get_state().await {
            Ok(rev) => rev,
            Err(e) => {
                if self.state_error {
                    debug!("Get state still failed: {e:#}");
                } else {
                    error!("Get state from server error: {e:#}");
                    self.state_error = true;
                }
                return;
            }
        };

        if self.state_error {
            info!("Server state error recovered");
            self.state_error = false;
        }

        if let Some(ref old_state) = self.state {
            if old_state.as_ref() == &latest_state {
                return;
            }
            debug!("Server state updated from {old_state:?} to {latest_state:?}");
        } else {
            debug!("First time getting server state {latest_state:?}");
        }

        self.state = Some(Arc::new(latest_state));
    }

    async fn handle_get_state(&mut self) -> (Option<Arc<ServerState>>, bool) {
        (self.state.clone(), self.state_error)
    }

    async fn handle_get_metadatas(&mut self, limit: u64) -> Result<ListResponse<Metadata>> {
        debug!("Get metadatas from server with limit: {}", limit);

        self.client
            .get_metadatas(GetMetadataRequest {
                query: QueryRequest {
                    limit: Some(limit),
                    ..Default::default()
                },
                ..Default::default()
            })
            .await
    }

    async fn handle_put_blob(&mut self, blob: Blob) -> Result<()> {
        debug!("Put blob to server: {:#?}", blob);
        self.client.put_blob(blob).await
    }

    async fn handle_get_blob(&mut self, id: u64) -> Result<Blob> {
        debug!("Get blob from server with id: {}", id);

        if let Some(cached) = self.blobs_cache.get(&id) {
            debug!("Get blob from cache: {id}");
            return Ok(cached.blob.clone());
        }

        debug!("Get blob from server: {id}");
        let blob = self.client.get_blob(id).await?;
        let now = Utc::now().timestamp() as u64;
        let cached = CacheBlob {
            blob: blob.clone(),
            expire: now + self.cache_seconds,
        };
        debug!("Cache blob {id}, it will expire at {}", cached.expire);
        self.blobs_cache.insert(id, cached);

        Ok(blob)
    }

    async fn handle_delete_blob(&mut self, id: u64) -> Result<()> {
        debug!("Delete blob from server with id: {}", id);

        self.blobs_cache.remove(&id);
        self.client.delete_blob(id).await
    }

    async fn handle_pin_blob(&mut self, id: u64, pin: bool) -> Result<()> {
        debug!("Pin blob from server with id: {} to {}", id, pin);
        self.client
            .patch_blob(PatchBlobRequest { id, pin: Some(pin) })
            .await
    }

    fn handle_recycle_cache(&mut self) {
        let now = Utc::now().timestamp() as u64;
        let mut expired = vec![];
        for (id, cache) in self.blobs_cache.iter() {
            if now > cache.expire {
                expired.push(*id);
            }
        }

        for id in expired {
            debug!("Recycle cache blob: {}", id);
            self.blobs_cache.remove(&id);
        }
    }
}
