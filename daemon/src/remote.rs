use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use csync_misc::api::blob::{Blob, PatchBlobRequest};
use csync_misc::api::metadata::{GetMetadataRequest, Metadata, Revision};
use csync_misc::api::{ListResponse, QueryRequest};
use csync_misc::client::restful::RestfulClient;
use log::{debug, error, info};
use tokio::select;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub struct Remote {
    get_revision_tx: mpsc::Sender<GetRevisionRequest>,
    get_metadatas_tx: mpsc::Sender<GetMetadatasRequest>,
    put_blob_tx: mpsc::Sender<PutBlobRequest>,
    get_blob_tx: mpsc::Sender<GetBlobRequest>,
    delete_blob_tx: mpsc::Sender<DeleteBlobRequest>,
    pin_blob_tx: mpsc::Sender<PinBlobRequest>,
}

impl Remote {
    pub fn start(client: RestfulClient, cache_seconds: u64) -> Self {
        let (get_revision_tx, get_revision_rx) = mpsc::channel(500);
        let (get_metadatas_tx, get_metadatas_rx) = mpsc::channel(500);
        let (put_blob_tx, put_blob_rx) = mpsc::channel(500);
        let (get_blob_tx, get_blob_rx) = mpsc::channel(500);
        let (delete_blob_tx, delete_blob_rx) = mpsc::channel(500);
        let (pin_blob_tx, pin_blob_rx) = mpsc::channel(500);

        let handler = RemoteHandler {
            client,
            revision: None,
            revision_error: false,
            blobs_cache: HashMap::new(),
            cache_seconds,
            get_revision_rx,
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
            get_revision_tx,
            get_metadatas_tx,
            put_blob_tx,
            get_blob_tx,
            delete_blob_tx,
            pin_blob_tx,
        }
    }

    pub async fn get_revision(&self) -> (Option<Arc<Revision>>, bool) {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.get_revision_tx
            .send(GetRevisionRequest { resp: resp_tx })
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

struct GetRevisionRequest {
    resp: oneshot::Sender<(Option<Arc<Revision>>, bool)>,
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
    revision: Option<Arc<Revision>>,
    revision_error: bool,

    blobs_cache: HashMap<u64, CacheBlob>,
    cache_seconds: u64,

    get_revision_rx: mpsc::Receiver<GetRevisionRequest>,
    get_metadatas_rx: mpsc::Receiver<GetMetadatasRequest>,
    put_blob_rx: mpsc::Receiver<PutBlobRequest>,
    get_blob_rx: mpsc::Receiver<GetBlobRequest>,
    delete_blob_rx: mpsc::Receiver<DeleteBlobRequest>,
    pin_blob_rx: mpsc::Receiver<PinBlobRequest>,
}

impl RemoteHandler {
    async fn main_loop(mut self) {
        let mut recycle_cache_intv = tokio::time::interval(Duration::from_secs(self.cache_seconds));
        let mut refresh_revision_intv = tokio::time::interval(Duration::from_secs(1));
        info!("Begin to handle remote requests");

        loop {
            select! {
                _ = refresh_revision_intv.tick() => {
                    self.refresh_revision().await;
                },

                Some(req) = self.get_revision_rx.recv() => {
                    let result = self.handle_get_revision().await;
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

    async fn refresh_revision(&mut self) {
        let latest_rev = match self.client.get_revision().await {
            Ok(rev) => rev,
            Err(e) => {
                if self.revision_error {
                    debug!("Get revision still failed: {e:#}");
                } else {
                    error!("Get revision from server error: {e:#}");
                    self.revision_error = true;
                }
                return;
            }
        };

        if self.revision_error {
            info!("Server revision error recovered");
            self.revision_error = false;
        }

        if let Some(ref old_rev) = self.revision {
            if old_rev.as_ref() == &latest_rev {
                return;
            }
            info!("Server revision updated from {old_rev:?} to {latest_rev:?}");
        } else {
            info!("First time getting server revision {latest_rev:?}");
        }

        self.revision = Some(Arc::new(latest_rev));
    }

    async fn handle_get_revision(&mut self) -> (Option<Arc<Revision>>, bool) {
        (self.revision.clone(), self.revision_error)
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
