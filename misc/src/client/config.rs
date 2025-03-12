use anyhow::{bail, Context, Result};
use log::info;
use serde::{Deserialize, Serialize};

use crate::client::restful::RestfulClientBuilder;
use crate::config::{CommonConfig, PathSet};
use crate::logs::LogsConfig;

use super::daemon::DaemonClient;
use super::restful::RestfulClient;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientConfig {
    #[serde(default = "ClientConfig::default_server")]
    pub server: String,

    #[serde(default = "ClientConfig::default_daemon_port")]
    pub daemon_port: u32,

    #[serde(default = "bool::default")]
    pub accept_invalid_certs: bool,

    #[serde(default = "ClientConfig::default_username")]
    pub username: String,

    #[serde(default = "ClientConfig::default_password")]
    pub password: String,

    #[serde(default)]
    pub logs: LogsConfig,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server: Self::default_server(),
            daemon_port: Self::default_daemon_port(),
            accept_invalid_certs: false,
            username: Self::default_username(),
            password: Self::default_password(),
            logs: LogsConfig::default(),
        }
    }
}

impl CommonConfig for ClientConfig {
    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if self.server.is_empty() {
            bail!("server address is required");
        }
        if self.daemon_port == 0 {
            bail!("daemon port is required");
        }
        if self.username.is_empty() {
            bail!("username is required");
        }
        if self.password.is_empty() {
            bail!("password is required");
        }

        self.logs.complete(ps).context("logs")?;

        Ok(())
    }
}

impl ClientConfig {
    pub async fn connect_restful(&self, use_token: bool) -> Result<RestfulClient> {
        info!("Connecting to restful server: {}", self.server);

        let client = RestfulClientBuilder::new(&self.server, &self.username, &self.password)
            .accept_invalid_certs(self.accept_invalid_certs)
            .use_token(use_token)
            .connect()
            .await?;

        Ok(client)
    }

    pub async fn connect_daemon(&self) -> Result<DaemonClient> {
        DaemonClient::connect(self.daemon_port).await
    }

    fn default_server() -> String {
        String::from("http://127.0.0.1:13577")
    }

    fn default_daemon_port() -> u32 {
        13578
    }

    fn default_username() -> String {
        String::from("admin")
    }

    fn default_password() -> String {
        String::from("admin_password123")
    }
}
