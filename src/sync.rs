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

// If the size of the text exceeds this value, it will be compared using SHA256
// hash calculation to reduce memory pressure.
const SHA256_TEXT_SIZE: usize = 1024 * 1024;

// If the size of the data in the clipboard exceeds this value, it will not be
// synchronized to prevent excessive pressure on the network and memory.
const DATA_MAX_SIZE: u64 = 32 << 20;

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
        let deserializer = &mut bincode::options()
            .with_fixint_encoding()
            .with_limit(DATA_MAX_SIZE);

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
        if buffer_size > DATA_MAX_SIZE {
            let size = human_bytes(buffer_size as u32);
            bail!("data is too huge {}, skip encoding", size);
        }
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

/// The purpose of the `start` function is to periodically check for changes in
/// the images and text stored in the clipboard. If any changes are detected,
/// the data is sent to the targets via socket for synchronization.
/// Additionally, it also needs to handle requests from external sources to modify
/// the clipboard. (from param `receiver`)
/// The clipboard checking and modification are performed synchronously, so there
/// is no need to worry about data consistency issues.
pub async fn start(cfg: config::Config, mut cb: Clipboard, mut receiver: mpsc::Receiver<Packet>) {
    let start = Instant::now();
    let interval = Duration::from_millis(cfg.interval);
    let mut intv = time::interval_at(start, interval);

    // Here we only need to store the hash value of the image, as comparing the
    // hash values is sufficient to determine if the image has changed.
    // Saving the entire image would consume too much memory.
    let current_image = get_clipboard_image(&mut cb);
    let mut current_image_hash = match current_image {
        Some(image) => Some(image.hash),
        None => None,
    };

    let mut current_text = get_clipboard_text(&mut cb);
    loop {
        tokio::select! {
            _ = intv.tick() => {
                let mut need_send_image = false;
                let mut image = get_clipboard_image(&mut cb);
                if let Some(image) = &image {
                    match &current_image_hash {
                        Some(current_hash) => {
                            if !current_hash.eq(&image.hash) {
                                need_send_image = true;
                            }
                        }
                        // If `current_image_hash` is `None`, it indicates that
                        // there is currently no image in the clipboard.
                        // In this case, we can send the image to the targets to
                        // complete the initial synchronization.
                        None => need_send_image = true,
                    }
                }
                if !need_send_image {
                    image = None;
                }

                let mut need_send_text = false;
                let mut text = get_clipboard_text(&mut cb);
                if let Some(text) = &text {
                    match &current_text {
                        Some(current) => {
                            // For comparing text, the logic is as follows: Only
                            // when the text is large (greater than the constant
                            // `SHA256_TEXT_SIZE`) is the hash value used for
                            // comparison. Otherwise, the text is compared directly.
                            // This is a compromise solution that takes into account
                            // both memory and CPU overhead.
                            if !current.eq(text) {
                                need_send_text = true;
                            }
                        }
                        None => need_send_text = true,
                    }
                }
                if !need_send_text {
                    text = None;
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
                    error!("Send packet error: {:#}", err);
                }
                let Packet { file: _, text, image } = packet;

                // Update current state
                if need_send_text {
                    current_text = text;
                }
                if need_send_image {
                    if let Some(image) = image {
                        current_image_hash = Some(image.hash);
                    }
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
                    let need_update = match current_image_hash.as_ref() {
                        Some(current_hash) => !current_hash.eq(&image.hash),
                        None => true,
                    };
                    if need_update {
                        set_clipboard_image(&mut cb, &image);
                        current_image_hash = Some(image.hash);
                    }
                }
                if let Some(file) = file {
                    sync_file(&cfg, file);
                }
            }
        }
    }
}

fn ignore_clipboard_error(err: &arboard::Error) -> bool {
    // If an encoding error occurs while reading the clipboard, it is ignored.
    // This is because `arboard` does not provide a universal method for reading
    // the clipboard, and we need to read both images and text simultaneously.
    // This can result in situations where we try to read text when the data in the
    // clipboard is actually an image.
    // Once the https://github.com/1Password/arboard/issues/11 is resolved, we will
    // have a more elegant way to handle this.
    match &err {
        arboard::Error::Unknown { description } => description == INCORRECT_CLIPBOARD_TYPE_ERROR,
        arboard::Error::ContentNotAvailable => true,
        _ => false,
    }
}

fn get_clipboard_text(cb: &mut Clipboard) -> Option<TextData> {
    match cb.get_text() {
        Ok(text) => {
            // Only when the text is large (greater than the constant
            // `SHA256_TEXT_SIZE`) is the SHA256 hash computed.
            let hash = if text.len() <= SHA256_TEXT_SIZE {
                None
            } else {
                Some(sha256::digest(text.as_str()))
            };
            Some(TextData { text, hash })
        }
        Err(err) => {
            if !ignore_clipboard_error(&err) {
                error!("Read text from clipboard error: {:#}", err);
            }
            None
        }
    }
}

fn set_clipboard_text(cb: &mut Clipboard, text: &String) {
    match cb.set_text(text) {
        Ok(_) => {}
        Err(err) => error!("Write text into clipboard error: {:#}", err),
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
            if !ignore_clipboard_error(&err) {
                error!("Read image from clipboard error: {:#}", err);
            }
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
        Err(err) => error!("Write image into clipboard error: {:#}", err),
    }
}

fn sync_file(cfg: &config::Config, file: FileData) {
    let path = cfg.dir.join(file.name);
    let dir = path.parent();
    if let Some(dir) = dir {
        match fs::read_dir(dir) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Err(err) = fs::create_dir_all(&dir) {
                    error!(r#"Create file dir "{}" error: {:#}"#, dir.display(), err);
                }
            }
            Err(err) => error!(r#"Read file dir "{}" error: {:#}"#, dir.display(), err),
            Ok(_) => {}
        }
    }

    let os_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        // TODO: won't work on Windows
        .mode(file.mode)
        .open(&path);
    if let Err(err) = os_file {
        error!(r#"Open file "{}" error: {:#}"#, path.display(), err);
        return;
    }
    let mut os_file = os_file.unwrap();

    if let Err(err) = os_file.write_all(&file.data) {
        error!(r#"Write file "{}" error: {}"#, path.display(), err);
        return;
    }
    info!(
        r#"Write {} to file "{}""#,
        human_bytes(file.data.len() as u32),
        path.display()
    );
}
