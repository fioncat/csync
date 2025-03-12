use std::cell::RefCell;
use std::sync::Mutex;

use csync_misc::api::metadata::{Metadata, ServerState};

use crate::auth::jwt::{JwtTokenGenerator, JwtTokenValidator};
use crate::config::ServerConfig;
use crate::db::Database;

pub struct ServerContext {
    pub db: Database,

    pub jwt_generator: JwtTokenGenerator,
    pub jwt_validator: JwtTokenValidator,

    pub cfg: ServerConfig,

    pub state: Mutex<RefCell<ServerState>>,
}

impl ServerContext {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            db: Database::new_test(),
            jwt_generator: JwtTokenGenerator::new_test(),
            jwt_validator: JwtTokenValidator::new_test(),
            cfg: ServerConfig::default(),
            state: Default::default(),
        }
    }

    pub fn grow_rev(&self) {
        let rev = self.state.lock().unwrap();
        let cur = rev.borrow().rev.unwrap_or(0);
        let new = cur + 1;
        rev.borrow_mut().rev = Some(new);
    }

    pub fn update_latest(&self, latest: Metadata) {
        let rev = self.state.lock().unwrap();
        let cur = rev.borrow().rev.unwrap_or(0);

        rev.replace(ServerState {
            rev: Some(cur + 1),
            latest: Some(latest),
        });
    }

    pub fn get_state(&self) -> ServerState {
        self.state.lock().unwrap().borrow().clone()
    }
}
