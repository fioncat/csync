use std::borrow::Cow;
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::os::unix::prelude::OpenOptionsExt;

use crate::client;
use crate::config;

use anyhow::bail;
use anyhow::Result;
use arboard::Clipboard;
use bincode::Options;
use human_bytes::human_bytes;
use serde::{Deserialize, Serialize};
use tokio::time::{self, Duration, Instant};
use tokio::{self, sync::mpsc};

use log::{error, info};

pub const CHANNEL_SIZE: usize = 512;

const SHA256_TEXT_SIZE: usize = 1024 * 1024;

const INCORRECT_CLIPBOARD_TYPE_ERROR: &str = "incorrect type received from clipboard";

#[derive(Debug, Deserialize, Serialize)]
pub struct Packet {
    pub file: Option<FileData>,
    pub text: Option<TextData>,
    pub image: Option<ImageData>,
}

impl Packet {
    const VERSION: u32 = 1;

    pub fn decode(data: &[u8]) -> Result<Packet> {
        const MAX_SIZE: u64 = 32 << 10; // 32 MiB

        let deserializer = &mut bincode::options()
            .with_fixint_encoding()
            .with_limit(MAX_SIZE);

        let version_size = deserializer.serialized_size(&Self::VERSION).unwrap() as _;
        if data.len() < version_size {
            bail!("could not decode packet: corrupted data");
        }
        let (bytes_version, bytes_data) = data.split_at(version_size);
        let version = deserializer.deserialize(bytes_version)?;

        let packet: Packet = match version {
            Self::VERSION => deserializer.deserialize(bytes_data)?,
            version => bail!("unsupported version {version}, supports: {}", Self::VERSION),
        };
        Ok(packet)
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let buffer_size =
            bincode::serialized_size(&Self::VERSION)? + bincode::serialized_size(self)?;
        let mut buffer = Vec::with_capacity(buffer_size as usize);

        bincode::serialize_into(&mut buffer, &Self::VERSION)?;
        bincode::serialize_into(&mut buffer, self)?;

        Ok(buffer)
    }
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut cmps = vec![];
        if let Some(text) = self.text.as_ref() {
            let bytes = human_bytes(text.text.len() as u32);
            cmps.push(format!("{} Text", bytes));
        }
        if let Some(image) = self.image.as_ref() {
            let bytes = human_bytes(image.data.len() as u32);
            cmps.push(format!("{} Image", bytes));
        }
        write!(f, "{{ {} }}", cmps.join(", "))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,

    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextData {
    pub text: String,
    pub hash: Option<String>,
}

impl TextData {
    fn eq(&self, other: &TextData) -> bool {
        match self.hash.as_ref() {
            Some(hash) => match other.hash.as_ref() {
                Some(o_hash) => hash == o_hash,
                None => false,
            },
            None => match other.hash {
                Some(_) => false,
                None => self.text == other.text,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileData {
    pub name: String,
    pub mode: u32,

    pub data: Vec<u8>,
}

pub async fn start(cfg: config::Config, mut cb: Clipboard, mut receiver: mpsc::Receiver<Packet>) {
    let start = Instant::now();
    let interval = Duration::from_millis(cfg.interval);
    let mut intv = time::interval_at(start, interval);

    let mut current_image = get_clipboard_image(&mut cb);
    let mut current_text = get_clipboard_text(&mut cb);
    loop {
        tokio::select! {
            _ = intv.tick() => {
                let mut need_send_image = false;
                let image = get_clipboard_image(&mut cb);
                if let Some(image) = &image {
                    match &current_image {
                        Some(current) => {
                            if current.hash != image.hash {
                                need_send_image = true;
                            }
                        }
                        None => need_send_image = true,
                    }
                }

                let mut need_send_text = false;
                let text = get_clipboard_text(&mut cb);
                if let Some(text) = &text {
                    match &current_text {
                        Some(current) => {
                            if !current.eq(text) {
                                need_send_text = true;
                            }
                        }
                        None => need_send_text = true,
                    }
                }

                if !need_send_text && !need_send_image {
                    continue;
                }
                let packet = Packet{
                    file: None,
                    text,
                    image,
                };
                if let Err(err) = client::send(&cfg, &packet).await {
                    error!("Send packet error: {}", err);
                }
                let Packet { file: _, text, image } = packet;
                if need_send_text {
                    current_text = text;
                }
                if need_send_image {
                    current_image = image;
                }
            }
            packet = receiver.recv() => {
                if let None = packet {
                    continue;
                }
                let packet = packet.unwrap();
                let Packet {file, text, image} = packet;
                if let Some(text) = text {
                    let need_update = match current_text.as_ref() {
                        Some(current) => !current.eq(&text),
                        None => true,
                    };
                    if need_update {
                        set_clipboard_text(&mut cb, &text.text);
                        current_text = Some(text);
                    }
                }
                if let Some(image) = image {
                    let need_update = match current_image.as_ref() {
                        Some(current) => current.hash != image.hash,
                        None => true,
                    };
                    if need_update {
                        set_clipboard_image(&mut cb, &image);
                        current_image = Some(image);
                    }
                }
                if let Some(file) = file {
                    sync_file(&cfg, file);
                }
            }
        }
    }
}

fn get_clipboard_text(cb: &mut Clipboard) -> Option<TextData> {
    match cb.get_text() {
        Ok(text) => {
            let hash = if text.len() <= SHA256_TEXT_SIZE {
                None
            } else {
                Some(sha256::digest(text.as_str()))
            };
            Some(TextData { text, hash })
        }
        Err(err) => {
            match &err {
                arboard::Error::Unknown { description } => {
                    if description == INCORRECT_CLIPBOARD_TYPE_ERROR {
                        return None;
                    }
                }
                arboard::Error::ContentNotAvailable => return None,
                _ => {}
            }
            error!("Read text from clipboard error: {}", err);
            None
        }
    }
}

fn set_clipboard_text(cb: &mut Clipboard, text: &String) {
    match cb.set_text(text) {
        Ok(_) => {}
        Err(err) => error!("Write text into clipboard error: {}", err),
    }
}

fn get_clipboard_image(cb: &mut Clipboard) -> Option<ImageData> {
    match cb.get_image() {
        Ok(image) => {
            let (width, height) = (image.width, image.height);
            let data = image.bytes.into_owned();
            let hash = sha256::digest::<&[u8]>(&data);
            Some(ImageData {
                width,
                height,
                hash,
                data,
            })
        }
        Err(err) => {
            match &err {
                arboard::Error::Unknown { description } => {
                    if description == INCORRECT_CLIPBOARD_TYPE_ERROR {
                        return None;
                    }
                }
                arboard::Error::ContentNotAvailable => return None,
                _ => {}
            }
            error!("Read image from clipboard error: {}", err);
            None
        }
    }
}

fn set_clipboard_image(cb: &mut Clipboard, image: &ImageData) {
    let cb_image = arboard::ImageData {
        width: image.width,
        height: image.height,
        bytes: Cow::from(&image.data),
    };
    match cb.set_image(cb_image) {
        Ok(_) => {}
        Err(err) => error!("Write image into clipboard error: {}", err),
    }
}

fn sync_file(cfg: &config::Config, file: FileData) {
    let path = cfg.dir.join(file.name);
    let dir = path.parent();
    if let Some(dir) = dir {
        match fs::read_dir(dir) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Err(err) = fs::create_dir_all(&dir) {
                    error!("Create file dir {} error: {}", dir.display(), err);
                }
            }
            Err(err) => error!("Read file dir {} error: {}", dir.display(), err),
            Ok(_) => {}
        }
    }

    let os_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(file.mode)
        .open(&path);
    if let Err(err) = os_file {
        error!("Open file {} error: {}", path.display(), err);
        return;
    }
    let mut os_file = os_file.unwrap();

    if let Err(err) = os_file.write_all(&file.data) {
        error!("Write file {} error: {}", path.display(), err);
        return;
    }
    info!(
        "Write {} to file {}",
        human_bytes(file.data.len() as u32),
        path.display()
    );
}
