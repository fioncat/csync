use std::path::PathBuf;

use anyhow::{bail, Error, Result};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    config::{Config, WriteConfig},
    utils,
};

struct Writer {
    download_dir: PathBuf,

    text_cmd: Option<Vec<String>>,
    image_cmd: Option<Vec<String>>,

    download_image: bool,

    digest: String,

    rx: Receiver<(Option<Vec<u8>>, String)>,

    err_tx: Sender<Error>,
}

impl Writer {
    fn new(
        cfg: &mut Config,
        rx: Receiver<(Option<Vec<u8>>, String)>,
        err_tx: Sender<Error>,
    ) -> Result<Self> {
        let write_cfg = cfg.get_write();
        let download_dir = Self::get_download_dir(&write_cfg)?;

        let download_image = write_cfg.download_image.unwrap_or(false);

        let digest = String::new();

        todo!()
    }

    fn get_download_dir(write_cfg: &WriteConfig) -> Result<PathBuf> {
        let dir = match write_cfg.download_dir.as_ref() {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => match dirs::download_dir() {
                Some(dir) => dir.join("csync"),
                None => bail!("no default download dir for your system, please config one"),
            },
        };

        utils::ensure_dir(&dir)?;
        Ok(dir)
    }
}
