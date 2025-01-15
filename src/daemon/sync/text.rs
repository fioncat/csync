use anyhow::{Ok, Result};
use async_trait::async_trait;

use crate::client::{Client, RequestError};
use crate::clipboard::Clipboard;
use crate::types::text::Text;

use super::{Resource, ResourceManager};

pub struct TextSyncManager;

#[async_trait]
impl ResourceManager for TextSyncManager {
    async fn read_server_hash(&self, client: &Client) -> Result<Option<String>> {
        let result: Result<Text, RequestError> =
            client.get_resource("texts", "latest".to_string()).await;
        if result.as_ref().is_err_and(|e| e.is_not_found()) {
            return Ok(None);
        }
        let hash = result?.hash;
        Ok(Some(hash))
    }

    async fn read_server(&self, client: &Client) -> Result<Option<Resource>> {
        let result = client.read_latest_text().await;
        if result.as_ref().is_err_and(|e| e.is_not_found()) {
            return Ok(None);
        }
        let text = result?;
        let rsc = Resource {
            data: text.content.unwrap().into_bytes(),
            hash: text.hash,
        };
        Ok(Some(rsc))
    }

    async fn write_server(&self, client: &Client, data: Vec<u8>) -> Result<()> {
        let text = String::from_utf8(data)?;
        client.put_text(text).await?;
        Ok(())
    }

    async fn read_cb(&self, cb: &Clipboard) -> Result<Option<Vec<u8>>> {
        let text = cb.read_text()?;
        Ok(text.map(|t| t.into_bytes()))
    }

    async fn write_cb(&self, cb: &Clipboard, data: Vec<u8>) -> Result<()> {
        let text = String::from_utf8(data)?;
        cb.write_text(text)
    }
}
