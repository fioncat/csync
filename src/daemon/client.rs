use anyhow::{bail, Result};
use reqwest::{Client, Method};

pub struct DaemonClient {
    url: String,
    client: Client,
}

impl DaemonClient {
    pub fn new(port: u16) -> Self {
        let url = format!("http://127.0.0.1:{port}");
        let client = Client::new();
        Self { url, client }
    }

    pub async fn send_data(&self, data: Vec<u8>) -> Result<()> {
        let req = self
            .client
            .request(Method::PUT, &self.url)
            .body(data)
            .build()?;

        let resp = self.client.execute(req).await?;
        if !resp.status().is_success() {
            bail!("daemon server returned bad status: {}", resp.status());
        }

        Ok(())
    }
}
