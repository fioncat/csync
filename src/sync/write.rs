use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::str::from_utf8;

use anyhow::{Context, Error, Result};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::config::{Config, WriteConfig};
use crate::net::frame::DataFrame;
use crate::utils::Cmd;

pub(super) fn start(cfg: &mut Config, err_tx: Sender<Error>) -> Result<Sender<DataFrame>> {
    let (tx, rx) = mpsc::channel::<DataFrame>(512);

    let mut writer = Writer::new(cfg, rx, err_tx)?;
    tokio::spawn(async move { writer.run().await });

    Ok(tx)
}

struct Writer {
    cfg: WriteConfig,

    rx: Receiver<DataFrame>,
    err_tx: Sender<Error>,

    image_path: Option<PathBuf>,
    download_path: PathBuf,
}

impl Writer {
    fn new(cfg: &mut Config, rx: Receiver<DataFrame>, err_tx: Sender<Error>) -> Result<Self> {
        let write_cfg = cfg.write.take().unwrap_or(WriteConfig {
            text_cmd: Vec::new(),
            image_cmd: Vec::new(),
            download_image: false,
        });

        let image_path = if write_cfg.download_image {
            Some(cfg.get_image_path())
        } else {
            None
        };

        let download_path = cfg.get_download_dir()?;

        Ok(Self {
            cfg: write_cfg,
            rx,
            err_tx,
            image_path,
            download_path,
        })
    }

    async fn run(&mut self) {
        loop {
            let frame = self.rx.recv().await.unwrap();
            println!();
            println!(
                "[From device {:?}: {} data]",
                frame.info.device,
                frame.body.len(),
            );

            if frame.info.file.is_some() {
                if let Err(err) = self.handle_file(frame) {
                    self.err_tx.send(err).await.unwrap();
                }
                continue;
            }

            let is_image = from_utf8(&frame.body).is_err();
            if is_image {
                if let Err(err) = self.handle_image(frame) {
                    self.err_tx.send(err).await.unwrap();
                }
                continue;
            }

            if let Err(err) = self.handle_text(frame) {
                self.err_tx.send(err).await.unwrap();
            }
        }
    }

    fn handle_file(&mut self, mut frame: DataFrame) -> Result<()> {
        let file_info = frame.info.file.take().unwrap();
        let path = self.download_path.join(&file_info.name);
        println!("<Download file to {}>", path.display());

        let mut opts = OpenOptions::new();
        opts.write(true).truncate(true).create(true);
        opts.mode(file_info.mode as u32);

        let mut file = opts
            .open(&path)
            .with_context(|| format!("open download file '{}'", path.display()))?;

        file.write_all(&frame.body)
            .with_context(|| format!("write data to file '{}'", path.display()))?;

        Ok(())
    }

    fn handle_image(&mut self, frame: DataFrame) -> Result<()> {
        if self.cfg.download_image {
            let path = self.image_path.as_ref().unwrap();
            println!("<Download image to {}>", path.display());
            fs::write(path, frame.body)
                .with_context(|| format!("write image data to '{}'", path.display()))?;
            return Ok(());
        }

        if !self.cfg.image_cmd.is_empty() {
            println!("<Execute image command>");
            let mut cmd = Cmd::new(&self.cfg.image_cmd, Some(frame.body), false);
            cmd.execute().context("execute image command")?;
            return Ok(());
        }

        println!("<Image data>");
        Ok(())
    }

    fn handle_text(&mut self, frame: DataFrame) -> Result<()> {
        if !self.cfg.text_cmd.is_empty() {
            println!("<Execute text command>");
            let mut cmd = Cmd::new(&self.cfg.text_cmd, Some(frame.body), false);
            cmd.execute().context("execute text command")?;
            return Ok(());
        }

        println!("{}", String::from_utf8_lossy(&frame.body));
        Ok(())
    }
}
