use actix_web::HttpRequest;
use anyhow::{bail, Result};

use super::token::TokenValidator;
use super::{Authenticator, AuthnResponse, AuthnUserInfo};

pub struct BearerTokenAuthenticator<T: TokenValidator> {
    validator: T,
}

impl<T: TokenValidator> BearerTokenAuthenticator<T> {
    pub fn new(validator: T) -> Self {
        Self { validator }
    }
}

impl<T: TokenValidator + Sync + Send> Authenticator for BearerTokenAuthenticator<T> {
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        _user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        let auth = match req.headers().get("Authorization") {
            Some(auth) => match auth.to_str() {
                Ok(auth) => auth.trim().to_string(),
                Err(_) => return Ok(AuthnResponse::Continue),
            },
            None => return Ok(AuthnResponse::Continue),
        };

        if auth.is_empty() {
            return Ok(AuthnResponse::Continue);
        }

        let mut iter = auth.split_whitespace();
        let bearer = iter.next();
        if bearer.is_none() {
            return Ok(AuthnResponse::Unauthenticated);
        }
        if bearer.unwrap().to_lowercase() != "bearer" {
            return Ok(AuthnResponse::Unauthenticated);
        }

        let token = match iter.next() {
            Some(token) => token,
            None => return Ok(AuthnResponse::Unauthenticated),
        };
        if token.is_empty() {
            return Ok(AuthnResponse::Unauthenticated);
        }

        let user = match self.validator.validate_token(token) {
            Ok(user) => user,
            Err(_) => return Ok(AuthnResponse::Unauthenticated),
        };
        if user.is_empty() {
            bail!("empty user identifier in token");
        }

        Ok(AuthnResponse::Ok(AuthnUserInfo {
            name: user,
            is_admin: false,
            is_anonymous: false,
        }))
    }
}
