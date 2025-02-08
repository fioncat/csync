use anyhow::{Ok, Result};
use async_trait::async_trait;
use csync_misc::client::{Client, RequestError};
use csync_misc::clipboard::Clipboard;
use csync_misc::types::image::Image;
use sha2::{Digest, Sha256};

use super::{Resource, ResourceManager};

pub struct ImageSyncManager;

#[async_trait]
impl ResourceManager for ImageSyncManager {
    async fn read_server_hash(&self, client: &Client) -> Result<Option<String>> {
        let result: Result<Image, RequestError> =
            client.get_resource("images", "latest".to_string()).await;
        if result.as_ref().is_err_and(|e| e.is_not_found()) {
            return Ok(None);
        }
        let hash = result?.hash;
        Ok(Some(hash))
    }

    async fn read_server(&self, client: &Client) -> Result<Option<Resource>> {
        let result = client.read_latest_image().await;
        if result.as_ref().is_err_and(|e| e.is_not_found()) {
            return Ok(None);
        }
        let data = result?;
        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);
        Ok(Some(Resource { data, hash }))
    }

    async fn write_server(&self, client: &Client, data: Vec<u8>) -> Result<()> {
        client.put_image(data).await?;
        Ok(())
    }

    async fn read_cb(&self, cb: &Clipboard) -> Result<Option<Vec<u8>>> {
        let data = cb.read_image()?;
        Ok(data)
    }

    async fn write_cb(&self, cb: &Clipboard, data: Vec<u8>) -> Result<()> {
        cb.write_image(data)
    }
}
