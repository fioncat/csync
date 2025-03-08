use csync_misc::api::metadata::Event;
use log::warn;
use tokio::sync::mpsc;

use crate::auth::jwt::{JwtTokenGenerator, JwtTokenValidator};
use crate::config::ServerConfig;
use crate::db::Database;

pub struct ServerContext {
    pub db: Database,

    pub jwt_generator: JwtTokenGenerator,
    pub jwt_validator: JwtTokenValidator,

    pub cfg: ServerConfig,

    pub event_tx: Option<mpsc::Sender<Event>>,
}

impl ServerContext {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            db: Database::new_test(),
            jwt_generator: JwtTokenGenerator::new_test(),
            jwt_validator: JwtTokenValidator::new_test(),
            cfg: ServerConfig::default(),
            event_tx: None,
        }
    }

    pub async fn notify_event(&self, event: Event) {
        if let Some(ref tx) = self.event_tx {
            if let Err(e) = tx.send(event).await {
                warn!("failed to notify event channel: {:#}", e);
            }
        }
    }
}
