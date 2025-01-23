use anyhow::Result;
use log::{info, warn};

use crate::filelock::{read_file_lock, write_file_lock};
use crate::time::current_timestamp;
use crate::types::token::TokenResponse;

use super::Client;

pub struct TokenFile {
    user: String,
    password: String,
    path: String,
}

impl TokenFile {
    pub fn new(user: String, password: String, path: String) -> Self {
        Self {
            user,
            password,
            path,
        }
    }

    pub async fn setup(&self, client: &mut Client) -> Result<()> {
        let token = match self.read()? {
            Some(token) => token,
            None => {
                info!("Logging to server...");
                let mut token_resp = client.login(&self.user, &self.password).await?;
                info!("Login success, save token to file");
                token_resp.expire_in -= Client::MAX_TIME_DELTA_WITH_SERVER;
                self.write(&token_resp)?;
                token_resp.token
            }
        };

        client.set_token(token);
        Ok(())
    }

    fn read(&self) -> Result<Option<String>> {
        let data = match read_file_lock(&self.path)? {
            Some(data) => data,
            None => return Ok(None),
        };

        let resp: TokenResponse = match serde_json::from_slice(&data) {
            Ok(resp) => resp,
            Err(_) => {
                warn!("Token file has invalid token data, we will ignore it");
                return Ok(None);
            }
        };

        if resp.user != self.user {
            warn!("Token file has different user, we will ignore it");
            return Ok(None);
        }

        let now = current_timestamp() as usize;
        if now >= resp.expire_in {
            info!("Token file has expired, we will acquire a new one");
            return Ok(None);
        }

        Ok(Some(resp.token))
    }

    fn write(&self, token: &TokenResponse) -> Result<()> {
        let data = serde_json::to_vec(token)?;
        write_file_lock(&self.path, &data)?;
        Ok(())
    }
}
