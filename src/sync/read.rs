use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Error, Result};
use tokio::select;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::time::{self, Instant};

use crate::config::{Config, ReadConfig};
use crate::sync::notify;
use crate::utils::{self, Cmd};

pub(super) struct ReadRequest {
    data_rx: Receiver<(Vec<u8>, String)>,
    digest_tx: Sender<UpdateDigestRequest>,
}

struct UpdateDigestRequest {
    digest: String,

    resp: oneshot::Sender<()>,
}

impl ReadRequest {
    pub(super) async fn recv_data(&mut self) -> Option<(Vec<u8>, String)> {
        self.data_rx.recv().await
    }

    pub(super) async fn update_digest(&mut self, digest: String) {
        let (resp_tx, resp_rx) = oneshot::channel::<()>();
        let req = UpdateDigestRequest {
            digest,
            resp: resp_tx,
        };

        self.digest_tx.send(req).await.unwrap();
        resp_rx.await.unwrap();
    }
}

pub(super) fn start(cfg: &mut Config, err_tx: Sender<Error>) -> Result<Option<ReadRequest>> {
    let (tx, rx) = mpsc::channel::<(Vec<u8>, String)>(512);
    let (digest_tx, digest_rx) = mpsc::channel::<UpdateDigestRequest>(512);

    let reader = Reader::new(cfg, tx, digest_rx, err_tx)?;
    if reader.is_none() {
        return Ok(None);
    }

    let mut reader = reader.unwrap();
    tokio::spawn(async move { reader.run().await });

    Ok(Some(ReadRequest {
        data_rx: rx,
        digest_tx,
    }))
}

struct Reader {
    cfg: ReadConfig,

    digest: String,
    dirty_digest: Option<String>,

    notify_path: Option<PathBuf>,

    tx: Sender<(Vec<u8>, String)>,
    digest_rx: Receiver<UpdateDigestRequest>,
    err_tx: Sender<Error>,
}

impl Reader {
    fn new(
        cfg: &mut Config,
        tx: Sender<(Vec<u8>, String)>,
        digest_rx: Receiver<UpdateDigestRequest>,
        err_tx: Sender<Error>,
    ) -> Result<Option<Self>> {
        let read_cfg = cfg.read.take();
        if read_cfg.is_none() {
            return Ok(None);
        }
        let read_cfg = read_cfg.unwrap();

        let notify_path = if read_cfg.notify {
            Some(cfg.get_notify_path())
        } else {
            None
        };

        Ok(Some(Self {
            cfg: read_cfg,
            digest: String::new(),
            dirty_digest: None,
            notify_path,
            tx,
            digest_rx,
            err_tx,
        }))
    }

    async fn run(&mut self) {
        let mut intv = time::interval_at(
            Instant::now(),
            Duration::from_millis(self.cfg.interval.into()),
        );

        loop {
            select! {
                _ = intv.tick() => {
                    self.handle_read().await;
                },
                Some(req) = self.digest_rx.recv() => {
                    self.handle_update_digest(req).await;
                },
            }
        }
    }

    async fn handle_read(&mut self) {
        let result = if self.cfg.notify {
            let path = self.notify_path.as_ref().unwrap();
            notify::read(path)
        } else {
            if self.cfg.cmd.is_empty() {
                unreachable!("this check should be done in Config::validate");
            }
            let result = Cmd::new(&self.cfg.cmd, None, true)
                .execute()
                .await
                .context("execute read command");
            if result.is_err() && self.cfg.allow_cmd_failure {
                Ok(None)
            } else {
                result
            }
        };
        if let Err(err) = result {
            self.err_tx.send(err).await.unwrap();
            return;
        }
        let data = result.unwrap();
        if data.is_none() {
            return;
        }
        let data = data.unwrap();
        if data.is_empty() {
            return;
        }

        let digest = utils::get_digest(&data);
        if let Some(dirty_digest) = self.dirty_digest.as_ref() {
            if dirty_digest == digest.as_str() {
                return;
            }
        }

        if digest == self.digest {
            return;
        }
        self.digest = digest.clone();
        self.dirty_digest = None;

        self.tx.send((data, digest)).await.unwrap();
    }

    async fn handle_update_digest(&mut self, req: UpdateDigestRequest) {
        let UpdateDigestRequest { digest, resp } = req;
        self.dirty_digest = Some(digest);
        resp.send(()).unwrap();
    }
}
