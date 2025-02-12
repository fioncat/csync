use std::cell::RefCell;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{bail, Result};
use csync_misc::client::factory::ClientFactory;
use csync_misc::client::Client;
use csync_misc::config::PathSet;
use csync_misc::humanize::human_bytes;
use csync_misc::imghdr::is_data_image;
use csync_misc::types::file::FileInfo;
use csync_misc::types::image::Image;
use csync_misc::types::request::Query;
use csync_misc::types::text::truncate_text;
use log::info;

use crate::sync::send::SyncSender;

use super::config::TrayAction;

pub struct ApiHandler {
    ps: PathSet,

    sync_tx: SyncSender,

    enable_text: bool,
    text_limit: u64,
    truncate_size: usize,
    text_action: Mutex<RefCell<TrayAction>>,

    enable_image: bool,
    image_limit: u64,
    image_action: Mutex<RefCell<TrayAction>>,

    enable_file: bool,
    file_limit: u64,
    file_action: Mutex<RefCell<TrayAction>>,

    auto_refresh: Mutex<RefCell<bool>>,
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

impl ApiHandler {
    pub fn new(ps: PathSet, sync_tx: SyncSender) -> Self {
        Self {
            ps,
            sync_tx,
            enable_text: false,
            text_limit: 0,
            truncate_size: 0,
            text_action: Mutex::new(RefCell::new(TrayAction::None)),
            enable_image: false,
            image_limit: 0,
            image_action: Mutex::new(RefCell::new(TrayAction::None)),
            enable_file: false,
            file_limit: 0,
            file_action: Mutex::new(RefCell::new(TrayAction::None)),
            auto_refresh: Mutex::new(RefCell::new(true)),
        }
    }

    pub fn with_text(&mut self, limit: u64, action: TrayAction) {
        self.enable_text = true;
        self.text_limit = limit;
        self.text_action.lock().unwrap().replace(action);
    }

    pub fn with_image(&mut self, limit: u64, action: TrayAction) {
        self.enable_image = true;
        self.image_limit = limit;
        self.image_action.lock().unwrap().replace(action);
    }

    pub fn with_file(&mut self, limit: u64, action: TrayAction) {
        self.enable_file = true;
        self.file_limit = limit;
        self.file_action.lock().unwrap().replace(action);
    }

    pub fn set_truncate_size(&mut self, size: usize) {
        self.truncate_size = size;
    }

    pub async fn build_menu(&self) -> Result<MenuData> {
        let client = self.build_client().await?;

        let mut data = MenuData {
            texts: vec![],
            images: vec![],
            files: vec![],
        };

        if self.enable_text {
            let query = Query {
                limit: Some(self.text_limit),
                ..Default::default()
            };

            let texts = client.read_texts(query).await?;
            for text in texts {
                let id = text.id;
                let text = truncate_text(text.content.unwrap(), self.truncate_size);
                let text = text.replace("\n", "\\n");
                data.texts.push(MenuTextItem { id, text });
            }
        }

        if self.enable_image {
            let query = Query {
                limit: Some(self.image_limit),
                ..Default::default()
            };

            let images: Vec<Image> = client.list_resources("images", query).await?;
            for image in images {
                let id = image.id;
                let size = human_bytes(image.size);
                data.images.push(MenuImageItem { id, size });
            }
        }

        if self.enable_file {
            let query = Query {
                limit: Some(self.file_limit),
                ..Default::default()
            };

            let files: Vec<FileInfo> = client.list_resources("files", query).await?;
            for file in files {
                let id = file.id;
                let name = file.name;
                let size = human_bytes(file.size);
                data.files.push(MenuFileItem { id, name, size });
            }
        }

        Ok(data)
    }

    pub async fn upload_text(&self, path: &Path) -> Result<()> {
        info!("Uploading text from file: {}", path.display());

        let text = fs::read_to_string(path)?;

        let client = self.build_client().await?;
        client.put_text(text).await?;

        Ok(())
    }

    pub async fn save_text(&self, id: u64, path: &Path) -> Result<()> {
        info!("Saving text {id} to file: {}", path.display());

        let client = self.build_client().await?;
        let text = client.read_text(id).await?;

        fs::write(path, text.content.unwrap())?;

        Ok(())
    }

    pub async fn copy_text(&self, id: u64) -> Result<()> {
        info!("Copying text {id} to clipboard");

        let client = self.build_client().await?;
        let text = client.read_text(id).await?;

        self.send_sync(text.content.unwrap().into_bytes()).await;

        Ok(())
    }

    pub async fn delete_text(&self, id: u64) -> Result<()> {
        info!("Deleting text {id}");

        let client = self.build_client().await?;
        client
            .delete_resource("texts", id.to_string().as_str())
            .await?;

        Ok(())
    }

    pub async fn upload_image(&self, path: &Path) -> Result<()> {
        info!("Uploading image from file: {}", path.display());

        let data = fs::read(path)?;
        if !is_data_image(&data) {
            bail!("file is not an image");
        }

        let client = self.build_client().await?;
        client.put_image(data).await?;

        Ok(())
    }

    pub async fn save_image(&self, id: u64, path: &Path) -> Result<()> {
        info!("Saving image {id} to file: {}", path.display());

        let client = self.build_client().await?;
        let data = client.read_image(id).await?;
        fs::write(path, data)?;

        Ok(())
    }

    pub async fn copy_image(&self, id: u64) -> Result<()> {
        info!("Copying image {id} to clipboard");

        let client = self.build_client().await?;
        let data = client.read_image(id).await?;

        self.send_sync(data).await;

        Ok(())
    }

    pub async fn delete_image(&self, id: u64) -> Result<()> {
        info!("Deleting image {id}");

        let client = self.build_client().await?;
        client
            .delete_resource("images", id.to_string().as_str())
            .await?;

        Ok(())
    }

    pub async fn upload_file(&self, path: &Path) -> Result<()> {
        info!("Uploading file from file: {}", path.display());

        let data = fs::read(path)?;
        let meta = fs::metadata(path)?;

        let name = match path.file_name() {
            Some(name) => match name.to_str() {
                Some(name) => name.to_string(),
                None => bail!("invalid file name"),
            },
            None => bail!("require file name"),
        };
        let mode = meta.mode() as u32;

        let client = self.build_client().await?;

        client.put_file(name, mode, data).await?;
        Ok(())
    }

    pub async fn save_file(&self, id: u64, path: &Path) -> Result<PathBuf> {
        info!("Saving file {id} to file: {}", path.display());

        let client = self.build_client().await?;
        let (info, data) = client.read_file(id).await?;

        let path = path.join(info.name);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .mode(info.mode)
            .open(&path)?;
        file.write_all(&data)?;

        Ok(path)
    }

    pub async fn copy_file(&self, id: u64) -> Result<()> {
        info!("Copying file {id} to clipboard");

        let client = self.build_client().await?;
        let (_, data) = client.read_file(id).await?;
        self.send_sync(data).await;

        Ok(())
    }

    pub async fn delete_file(&self, id: u64) -> Result<()> {
        info!("Deleting file {id}");

        let client = self.build_client().await?;
        client
            .delete_resource("files", id.to_string().as_str())
            .await?;

        Ok(())
    }

    #[allow(clippy::needless_bool)]
    pub fn get_auto_refresh(&self) -> bool {
        let auto_refresh = self.auto_refresh.lock().unwrap();
        if *auto_refresh.borrow() {
            true
        } else {
            false
        }
    }

    pub fn update_auto_refresh(&self) {
        let current = self.get_auto_refresh();
        self.auto_refresh.lock().unwrap().replace(!current);
    }

    pub fn get_tmp_path(&self, name: &str) -> PathBuf {
        if name.is_empty() {
            return self.ps.tmp_path.clone();
        }
        self.ps.tmp_path.join(name)
    }

    pub fn get_config_path(&self, name: &str) -> PathBuf {
        self.ps.config_path.join(format!("{name}.toml"))
    }

    pub fn set_text_action(&self, action: TrayAction) {
        info!("Setting text action: {:?}", action);
        self.text_action.lock().unwrap().replace(action);
    }

    pub fn set_image_action(&self, action: TrayAction) {
        info!("Setting image action: {:?}", action);
        self.image_action.lock().unwrap().replace(action);
    }

    pub fn set_file_action(&self, action: TrayAction) {
        info!("Setting file action: {:?}", action);
        self.file_action.lock().unwrap().replace(action);
    }

    pub fn get_text_action(&self) -> TrayAction {
        *self.text_action.lock().unwrap().borrow()
    }

    pub fn get_image_action(&self) -> TrayAction {
        *self.image_action.lock().unwrap().borrow()
    }

    pub fn get_file_action(&self) -> TrayAction {
        *self.file_action.lock().unwrap().borrow()
    }

    async fn build_client(&self) -> Result<Client> {
        let client_factory = ClientFactory::load(&self.ps)?;
        client_factory.build_client_with_token_file().await
    }

    async fn send_sync(&self, data: Vec<u8>) {
        let mut sync_tx = self.sync_tx.clone();
        sync_tx.send(data).await;
    }
}
