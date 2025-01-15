use anyhow::Result;

use crate::client::token::TokenFile;
use crate::config::{CommonConfig, PathSet};
use crate::secret::factory::SecretFactory;

use super::config::ClientConfig;
use super::Client;

pub struct ClientFactory {
    cfg: ClientConfig,
}

impl ClientFactory {
    pub fn new(cfg: ClientConfig) -> Self {
        Self { cfg }
    }

    pub fn load(ps: &PathSet) -> Result<Self> {
        let cfg = ps.load_config("client", ClientConfig::default)?;
        Ok(Self { cfg })
    }

    pub async fn build_client_with_token_file(&self) -> Result<Client> {
        let mut client = self.build_client().await?;
        let token_file = TokenFile::new(
            self.cfg.user.clone(),
            self.cfg.password.clone(),
            self.cfg.token_path.clone(),
        );
        token_file.setup(&mut client).await?;

        Ok(client)
    }

    pub async fn build_client(&self) -> Result<Client> {
        let mut client = Client::connect(&self.cfg.server, &self.cfg.cert_path).await?;

        let secret_factory = SecretFactory::new();
        let secret = secret_factory.build_secret(&self.cfg.secret)?;
        if let Some(secret) = secret {
            client.set_secret(secret);
        }

        Ok(client)
    }

    pub fn config(&self) -> &ClientConfig {
        &self.cfg
    }
}
