use std::collections::HashSet;
use std::sync::Arc;

use actix_web::HttpRequest;
use log::{info, warn};

use crate::server::authn::token::jwt::JwtTokenGenerator;
use crate::server::authn::token::TokenGenerator;
use crate::server::db::Database;
use crate::server::response::{self, Response};
use crate::types::user::User;

use super::Handler;

pub struct LoginHandler {
    admin_password: Option<String>,
    admin_allow_list: HashSet<String>,
    token_generator: JwtTokenGenerator,
    db: Arc<Database>,
}

impl LoginHandler {
    pub fn new(
        admin_password: Option<String>,
        admin_allow_list: HashSet<String>,
        token_generator: JwtTokenGenerator,
        db: Arc<Database>,
    ) -> Self {
        Self {
            admin_password,
            admin_allow_list,
            token_generator,
            db,
        }
    }
}

impl Handler for LoginHandler {
    fn handle(&self, path: &str, req: HttpRequest, body: Option<Vec<u8>>) -> Response {
        let path = path.trim_end_matches('/');
        let name = String::from(path);

        if name.is_empty() {
            return Response::bad_request("User name is required");
        }

        let password = match body {
            Some(body) => match String::from_utf8(body) {
                Ok(password) => password,
                Err(_) => {
                    return Response::bad_request("Invalid password");
                }
            },
            None => {
                return Response::bad_request("Password is required");
            }
        };

        if name == "admin" {
            if self.admin_allow_list.is_empty() {
                return Response::unauthenticated("Admin is disabled");
            }
            match self.admin_password {
                Some(ref admin_password) => {
                    if admin_password != &password {
                        return Response::unauthenticated("Invalid admin password");
                    }

                    let token = match self.token_generator.generate_token(name) {
                        Ok(token) => token,
                        Err(e) => {
                            log::error!("Failed to generate admin token: {e:#}");
                            return Response::error(response::TOKEN_ERROR);
                        }
                    };

                    let client_ip = req
                        .connection_info()
                        .peer_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_default();

                    let allow = self.admin_allow_list.contains("*")
                        || self.admin_allow_list.contains(&client_ip);
                    if !allow {
                        warn!("Admin login request: password corrected, but was rejected by client IP '{client_ip}', your admin password may have been leaked, please consider change or disable it");
                        return Response::unauthenticated("ClientIP blocked");
                    }

                    info!("Admin login succeeded, from '{client_ip}'");
                    return Response::json(token);
                }
                None => return Response::unauthenticated("Admin is disabled"),
            }
        }

        let result = self.db.with_transaction(|tx, _cache| {
            if !tx.is_user_exists(&name)? {
                return Ok(None);
            }

            let record = tx.get_user(&name)?;
            Ok(Some(record))
        });
        let record = match result {
            Ok(Some(record)) => record,
            Ok(None) => {
                return Response::unauthenticated("User not found");
            }
            Err(e) => {
                log::error!("Failed to get user record for login: {e:#}");
                return Response::error(response::DATABASE_ERROR);
            }
        };

        let input_hash = User::get_password_hash(&password, &record.salt);
        if input_hash != record.hash {
            return Response::unauthenticated("Invalid password");
        }

        let token = match self.token_generator.generate_token(name) {
            Ok(token) => token,
            Err(e) => {
                log::error!("Failed to generate token: {e:#}");
                return Response::error(response::TOKEN_ERROR);
            }
        };
        Response::json(token)
    }
}
