use anyhow::{bail, Context, Error, Result};
use csync_clipboard::Clipboard;
use csync_proto::client::{Client, TerminalPassword};
use csync_proto::frame::DataFrame;
use tokio::select;
use tokio::sync::mpsc::Sender;
use tokio::time::{self, Duration, Instant};

use crate::config::{Config, Target};
use crate::output::Output;

pub struct Sync {
    readonly: bool,

    pull_intv: Duration,
    start: Instant,

    cb: Clipboard,
    output: Output,
    client: Client<TerminalPassword>,
}

impl Sync {
    pub async fn new(cfg: &Config) -> Result<Sync> {
        let target = Target::parse(&cfg.target)?;
        let mut readonly = false;
        if let None = target.subs {
            readonly = true;
        }

        let mut writeonly = false;
        if let None = target.publish {
            writeonly = true;
        }

        let cb = Clipboard::new(readonly, writeonly).context("init clipboard")?;

        let output = Output::new(&cfg, &target);
        let client = target.build_client(&cfg).await?;

        if cfg.pull_interval < 100 || cfg.pull_interval > 20000 {
            bail!("invalid pull_interval, should be in range [100, 20000]")
        }
        let pull_intv = Duration::from_millis(cfg.pull_interval as u64);
        let start = Instant::now();

        Ok(Sync {
            readonly,
            pull_intv,
            start,
            cb,
            output,
            client,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if let None = self.cb.read_rx {
            let mut write_tx = self.cb.write_tx.take().unwrap();
            let mut pull_intv = time::interval_at(self.start, self.pull_intv);

            loop {
                select! {
                    _ = pull_intv.tick() => {
                        self.handle_sub(&mut write_tx).await?;
                    }
                    Some(err) = self.cb.error_rx.recv() => {
                        self.handle_error(err)?;
                    }
                }
            }
        }

        if let None = self.cb.write_tx {
            let mut read_rx = self.cb.read_rx.take().unwrap();
            loop {
                select! {
                    Some(frame) = read_rx.recv() => {
                        self.handle_publish(frame).await?;
                    }
                    Some(err) = self.cb.error_rx.recv() => {
                        self.handle_error(err)?;
                    }
                }
            }
        }

        let mut write_tx = self.cb.write_tx.take().unwrap();
        let mut read_rx = self.cb.read_rx.take().unwrap();
        let mut pull_intv = time::interval_at(self.start, self.pull_intv);

        loop {
            select! {
                _ = pull_intv.tick() => {
                    self.handle_sub(&mut write_tx).await?;
                }
                Some(frame) = read_rx.recv() => {
                    self.handle_publish(frame).await?;
                }
                Some(err) = self.cb.error_rx.recv() => {
                    self.handle_error(err)?;
                }
            }
        }
    }

    async fn handle_publish(&mut self, frame: Option<DataFrame>) -> Result<()> {
        if let None = frame {
            return Ok(());
        }
        let frame = frame.unwrap();

        self.output.show(&frame);
        self.client
            .push(frame)
            .await
            .context("push data to server")?;
        Ok(())
    }

    async fn handle_sub(&mut self, write_tx: &mut Sender<DataFrame>) -> Result<()> {
        let frame = self.client.pull().await.context("pull data from server")?;
        if let None = frame {
            return Ok(());
        }
        let frame = frame.unwrap();

        self.output.show(&frame);
        if self.readonly {
            return Ok(());
        }

        write_tx.send(frame).await.unwrap();

        Ok(())
    }

    fn handle_error(&self, err: Error) -> Result<()> {
        bail!("server error: {:#}", err)
    }
}
