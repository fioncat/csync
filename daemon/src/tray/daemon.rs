use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use anyhow::{Context, Result};
use csync_misc::client::Client;
use csync_misc::humanize::human_bytes;
use csync_misc::types::file::FileInfo;
use csync_misc::types::image::Image;
use csync_misc::types::request::Query;
use csync_misc::types::text::{truncate_text, Text};
use csync_misc::types::token::TokenResponse;
use log::{error, info};
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::now::current_timestamp;
use crate::sync::send::SyncSender;

pub struct TrayDaemon {
    pub latest_text_id: Option<u64>,
    pub latest_image_id: Option<u64>,
    pub latest_file_id: Option<u64>,

    pub client: Client,

    pub sync_tx: SyncSender,

    pub token: Option<TokenResponse>,
    pub user: String,
    pub password: String,

    pub menu_tx: mpsc::Sender<MenuData>,
    pub write_rx: mpsc::Receiver<WriteRequest>,

    pub limit: u64,
    pub truncate_size: usize,
}

pub struct MenuData {
    pub texts: Vec<MenuTextItem>,
    pub images: Vec<MenuImageItem>,
    pub files: Vec<MenuFileItem>,
}

pub struct MenuTextItem {
    pub id: u64,
    pub text: String,
}

pub struct MenuImageItem {
    pub id: u64,
    pub size: String,
}

pub struct MenuFileItem {
    pub id: u64,
    pub name: String,
    pub size: String,
}

#[derive(Debug)]
pub enum WriteRequest {
    Text(u64),
    Image(u64, Option<String>),
    File(u64, Option<String>),
}

impl TrayDaemon {
    const REFRESH_MENU_INTERVAL: Duration = Duration::from_secs(1);

    pub async fn run(mut self) {
        info!("Starting tray daemon");
        let mut intv = interval(Self::REFRESH_MENU_INTERVAL);
        loop {
            select! {
                _ = intv.tick() => {
                    let need_refresh = match self.need_refresh_menu().await {
                        Ok(need_refresh) => need_refresh,
                        Err(e) => {
                            error!("Check need refresh menu error: {:#}", e);
                            continue;
                        }
                    };
                    if need_refresh {
                        let menu = match self.build_menu().await {
                            Ok(menu) => menu,
                            Err(e) => {
                                error!("Build menu error: {:#}", e);
                                continue;
                            }
                        };
                        self.menu_tx.send(menu).await.unwrap();
                    }
                },

                Some(req) = self.write_rx.recv() => {
                    match req {
                        WriteRequest::Text(id) => {
                            if let Err(e) = self.write_text(id).await {
                                error!("Write text error: {:#}", e);
                            }
                        }
                        WriteRequest::Image(id, path) => {
                            if let Err(e) = self.write_image(id, path).await {
                                error!("Write image error: {:#}", e);
                            }
                        }
                        WriteRequest::File(id, path) => {
                            if let Err(e) = self.write_file(id, path).await {
                                error!("Write file error: {:#}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn write_text(&mut self, id: u64) -> Result<()> {
        info!("Received text write request {id}, sending to daemon server");
        self.refresh_token().await?;
        let text = self.client.read_text(id).await?;
        let text = text.content.unwrap();

        self.sync_tx.send(text.into_bytes()).await;
        Ok(())
    }

    async fn write_image(&mut self, id: u64, path: Option<String>) -> Result<()> {
        info!("Received image write request {id}");
        self.refresh_token().await?;
        let data = self.client.read_image(id).await?;

        match path {
            Some(path) => {
                info!("Writing image {id} to file {path}");
                fs::write(path, data).context("write image data")?;
                Ok(())
            }
            None => {
                info!("Sending image {id} to daemon server");
                self.sync_tx.send(data).await;
                Ok(())
            }
        }
    }

    async fn write_file(&mut self, id: u64, path: Option<String>) -> Result<()> {
        info!("Received file write request {id}");
        self.refresh_token().await?;
        let (info, data) = self.client.read_file(id).await?;

        match path {
            Some(path) => {
                info!("Writing file {id} to file {path}");
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(info.mode)
                    .open(path)
                    .context("failed to open file")?;
                file.write_all(&data).context("failed to write file")?;
                Ok(())
            }
            None => {
                info!("Sending file {id} to daemon server");
                self.sync_tx.send(data).await;
                Ok(())
            }
        }
    }

    async fn need_refresh_menu(&mut self) -> Result<bool> {
        self.refresh_token().await?;

        let latest_text = self
            .client
            .get_resource_option::<Text>("texts", String::from("latest"))
            .await?
            .map(|t| t.id);
        if self.latest_text_id != latest_text {
            info!(
                "Latest text changed to {:?}, need refresh menu",
                latest_text
            );
            self.latest_text_id = latest_text;
            return Ok(true);
        }

        let latest_image = self
            .client
            .get_resource_option::<Image>("images", String::from("latest"))
            .await?
            .map(|i| i.id);
        if self.latest_image_id != latest_image {
            info!(
                "Latest image changed to {:?}, need refresh menu",
                latest_image
            );
            self.latest_image_id = latest_image;
            return Ok(true);
        }

        let latest_file = self
            .client
            .get_resource_option::<FileInfo>("files", String::from("latest"))
            .await?
            .map(|f| f.id);
        if self.latest_file_id != latest_file {
            info!(
                "Latest file changed to {:?}, need refresh menu",
                latest_file
            );
            self.latest_file_id = latest_file;
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn build_menu(&mut self) -> Result<MenuData> {
        self.refresh_token().await?;

        let mut data = MenuData {
            texts: vec![],
            images: vec![],
            files: vec![],
        };

        let query = Query {
            limit: Some(self.limit),
            ..Default::default()
        };

        let texts = self.client.read_texts(query.clone()).await?;
        for text in texts {
            let id = text.id;
            let text = truncate_text(text.content.unwrap(), self.truncate_size);
            let text = text.replace("\n", "\\n");
            data.texts.push(MenuTextItem { id, text });
        }

        let images: Vec<Image> = self.client.list_resources("images", query.clone()).await?;
        for image in images {
            let id = image.id;
            let size = human_bytes(image.size);
            data.images.push(MenuImageItem { id, size });
        }

        let files: Vec<FileInfo> = self.client.list_resources("files", query).await?;
        for file in files {
            let id = file.id;
            let name = file.name;
            let size = human_bytes(file.size);
            data.files.push(MenuFileItem { id, name, size });
        }

        Ok(data)
    }

    async fn refresh_token(&mut self) -> Result<()> {
        let mut need_flush = true;
        if let Some(ref token) = self.token {
            let now = current_timestamp() as usize;
            if now < token.expire_in {
                need_flush = false;
            }
        }

        if !need_flush {
            return Ok(());
        }

        info!("Refreshing client token");
        let mut resp = self.client.login(&self.user, &self.password).await?;
        self.client.set_token(resp.token.clone());

        resp.expire_in -= Client::MAX_TIME_DELTA_WITH_SERVER;
        info!("Token refreshed, expire_in: {}", resp.expire_in);

        self.token = Some(resp);
        Ok(())
    }
}
