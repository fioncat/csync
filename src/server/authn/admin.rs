use std::collections::HashSet;

use actix_web::HttpRequest;
use anyhow::Result;

use super::{Authenticator, AuthnResponse, AuthnUserInfo};

pub struct AdminAuthenticator {
    allow_list: HashSet<String>,
}

impl AdminAuthenticator {
    pub fn new(allow_list: HashSet<String>) -> Self {
        Self { allow_list }
    }
}

impl Authenticator for AdminAuthenticator {
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        if user.is_none() {
            return Ok(AuthnResponse::Continue);
        }

        let mut user = user.unwrap();
        if user.name != "admin" {
            return Ok(AuthnResponse::Ok(user));
        }

        if self.allow_list.contains("*") {
            user.is_admin = true;
            return Ok(AuthnResponse::Ok(user));
        }

        let conn_info = req.connection_info();
        let addr = match conn_info.peer_addr() {
            Some(addr) => addr,
            None => return Ok(AuthnResponse::Unauthenticated),
        };
        if !self.allow_list.contains(addr) {
            return Ok(AuthnResponse::Unauthenticated);
        }

        user.is_admin = true;
        Ok(AuthnResponse::Ok(user))
    }
}
