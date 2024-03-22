use std::time::Duration;

use anyhow::{bail, Error, Result};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::{self, Instant};

use crate::config::Config;
use crate::utils::{self, Cmd};

fn start(cfg: &mut Config, err_tx: Sender<Error>) -> Result<Receiver<(Vec<u8>, String)>> {
    let (tx, rx) = mpsc::channel::<(Vec<u8>, String)>(512);

    let reader = Reader::new(cfg, tx, err_tx)?;
    if reader.is_none() {
        return Ok(rx);
    }

    let mut reader = reader.unwrap();
    tokio::spawn(async move {
        reader.run().await;
    });

    Ok(rx)
}

struct Reader {
    cmd: Vec<String>,
    digest: String,
    tx: Sender<(Vec<u8>, String)>,
    interval: u32,

    err_tx: Sender<Error>,
}

impl Reader {
    const DEFAULT_INTERVAL: u32 = 200;

    const MIN_INTERVAL: u32 = 100;
    const MAX_INTERVAL: u32 = 5000;

    fn new(
        cfg: &mut Config,
        tx: Sender<(Vec<u8>, String)>,
        err_tx: Sender<Error>,
    ) -> Result<Option<Self>> {
        let read_cfg = cfg.get_read();
        if read_cfg.is_none() {
            return Ok(None);
        }

        let read_cfg = read_cfg.unwrap();
        if read_cfg.cmd.is_none() {
            return Ok(None);
        }

        let interval = match read_cfg.interval {
            Some(interval) => {
                if !(Self::MIN_INTERVAL..=Self::MAX_INTERVAL).contains(&interval) {
                    bail!(
                        "invalid read interval in config, should be in [{}, {}]",
                        Self::MIN_INTERVAL,
                        Self::MAX_INTERVAL
                    );
                }

                interval
            }
            None => Self::DEFAULT_INTERVAL,
        };

        let cmd = read_cfg.cmd.unwrap();

        Ok(Some(Self {
            cmd,
            digest: String::new(),
            tx,
            interval,
            err_tx,
        }))
    }

    async fn run(&mut self) {
        let mut intv =
            time::interval_at(Instant::now(), Duration::from_millis(self.interval as u64));

        loop {
            intv.tick().await;

            let mut cmd = Cmd::new(&self.cmd, None, true);
            let result = cmd.execute();
            if let Err(err) = result {
                self.err_tx.send(err).await.unwrap();
                return;
            };
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
