use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Error, Result};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;
use tokio::time::{self, Instant};

use crate::config::{Config, ReadConfig};
use crate::utils::{self, Cmd};

struct ReadController {}

struct UpdateDigestRequest {
    digest: String,
}

fn start(cfg: &mut Config, err_tx: Sender<Error>) -> Result<Receiver<(Vec<u8>, String)>> {
    let (tx, rx) = mpsc::channel::<(Vec<u8>, String)>(512);
    let (digest_tx, digest_rx) = mpsc::channel::<String>(512);

    let reader = Reader::new(cfg, tx, digest_rx, err_tx)?;
    if reader.is_none() {
        return Ok(rx);
    }

    let mut reader = reader.unwrap();
    tokio::spawn(async move { reader.run().await });

    Ok(rx)
}

struct Reader {
    cfg: ReadConfig,

    digest: String,

    notify_path: Option<PathBuf>,

    tx: Sender<(Vec<u8>, String)>,
    digest_rx: Receiver<String>,
    err_tx: Sender<Error>,
}

impl Reader {
    fn new(
        cfg: &mut Config,
        tx: Sender<(Vec<u8>, String)>,
        digest_rx: Receiver<String>,
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
            intv.tick().await;

            let result = if self.cfg.notify {
                let path = self.notify_path.as_ref().unwrap();
                utils::read_filelock(path).context("read notify file")
            } else {
                if self.cfg.cmd.is_empty() {
                    unreachable!("this check should be done in Config::validate");
                }
                Cmd::new(&self.cfg.cmd, None, true).execute()
            };
            if let Err(err) = result {
                self.err_tx.send(err).await.unwrap();
                return;
            }
            let data = result.unwrap();
            if data.is_none() {
                continue;
            }
            let data = data.unwrap();
            if data.is_empty() {
                continue;
            }

            let digest = utils::get_digest(&data);
            if digest == self.digest {
                continue;
            }

            self.tx.send((data, digest)).await.unwrap();
        }
    }
}
