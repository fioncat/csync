mod basic;
mod bearer_token;

pub mod jwt;
pub mod rsa;

use actix_web::HttpRequest;
use csync_misc::api;
use csync_misc::api::user::User;

use crate::context::ServerContext;

#[macro_export]
macro_rules! auth_request {
    ($sc:expr, $req:expr) => {
        match $crate::auth::auth_request_raw($sc, &$req) {
            $crate::auth::AuthResult::Ok(user) => user,
            $crate::auth::AuthResult::Failed(msg) => {
                return csync_misc::api::Response::unauthorized(msg)
            }
        }
    };
}

pub enum AuthResult {
    Ok(User),
    Failed(String),
}

pub fn auth_request_raw(sc: &ServerContext, req: &HttpRequest) -> AuthResult {
    let auth_header = match req.headers().get(api::HEADER_AUTHORIZATION) {
        Some(header) => match header.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return AuthResult::failed("invalid authorization header value"),
        },
        None => return AuthResult::failed("missing authorization"),
    };

    let fields = auth_header.split_whitespace().collect::<Vec<&str>>();
    if fields.len() != 2 {
        return AuthResult::failed("invalid authorization header format");
    }

    let auth_type = fields[0];
    let auth = fields[1].to_string();

    let is_remote = if let Some(addr) = req.connection_info().peer_addr() {
        addr != "127.0.0.1"
    } else {
        true
    };

    match auth_type.to_lowercase().as_str() {
        "basic" => match basic::auth_basic(sc, auth, is_remote) {
            Ok(user) => AuthResult::Ok(user),
            Err(e) => AuthResult::Failed(format!("basic auth failed: {e:#}")),
        },
        "bearer" => match bearer_token::auth_bearer_token(sc, auth, is_remote) {
            Ok(user) => AuthResult::Ok(user),
            Err(e) => AuthResult::Failed(format!("bearer token auth failed: {e:#}")),
        },
        _ => AuthResult::failed("unsupported authorization type"),
    }
}

impl AuthResult {
    fn failed(msg: impl ToString) -> Self {
        Self::Failed(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use actix_web::test::TestRequest;
    use chrono::Utc;
    use csync_misc::api::user::PutUserRequest;
    use csync_misc::api::Response;
    use csync_misc::code;

    use crate::db::types::CreateUserParams;

    use super::*;

    fn test_handler(req: HttpRequest, sc: &ServerContext, expect_user: User) -> Response<()> {
        let user = auth_request!(sc, req);
        assert_eq!(user, expect_user);
        Response::ok()
    }

    fn test_auth(auth: &str, sc: &ServerContext, expect_user: User, remote: bool) -> Response<()> {
        let peer = if remote {
            "86.12.34.10:1234"
        } else {
            "127.0.0.1:1234"
        };
        let peer: SocketAddr = peer.parse().unwrap();

        let req = TestRequest::default()
            .insert_header((api::HEADER_AUTHORIZATION, auth))
            .peer_addr(peer)
            .to_http_request();
        test_handler(req, sc, expect_user)
    }

    #[test]
    fn test_auth_request() {
        let sc = ServerContext::new_test();

        sc.db
            .with_transaction(|tx| {
                tx.create_user(CreateUserParams {
                    user: PutUserRequest {
                        name: "test_user".to_string(),
                        password: code::sha256("test_passwordtest_salt"),
                        admin: false,
                    },
                    salt: "test_salt".to_string(),
                    update_time: 0,
                })?;
                Ok(())
            })
            .unwrap();

        let basic_auth = format!("Basic test_user:{}", code::base64_encode("test_password"));
        let user = User {
            name: "test_user".to_string(),
            admin: false,
            update_time: 0,
        };
        let resp = test_auth(&basic_auth, &sc, user.clone(), true);
        assert_eq!(resp.code, 200);

        let resp = test_auth(&basic_auth, &sc, user.clone(), false);
        assert_eq!(resp.code, 200);

        let admin_auth = format!("Basic admin:{}", code::base64_encode("admin_password123"));
        let admin_user = User {
            name: "admin".to_string(),
            admin: true,
            update_time: 0,
        };
        let resp = test_auth(&admin_auth, &sc, admin_user.clone(), false);
        assert_eq!(resp.code, 200);

        let resp = test_auth(&admin_auth, &sc, admin_user.clone(), true);
        assert_eq!(resp.code, 401);

        let now = Utc::now().timestamp() as u64;
        let user_token = sc.jwt_generator.generate_token(user.clone(), now).unwrap();
        let admin_token = sc
            .jwt_generator
            .generate_token(admin_user.clone(), now)
            .unwrap();

        let user_token_auth = format!("Bearer {}", user_token.token);
        let admin_token_auth = format!("Bearer {}", admin_token.token);

        let resp = test_auth(&user_token_auth, &sc, user.clone(), true);
        assert_eq!(resp.code, 200);

        let resp = test_auth(&user_token_auth, &sc, user.clone(), false);
        assert_eq!(resp.code, 200);

        let resp = test_auth(&admin_token_auth, &sc, admin_user.clone(), false);
        assert_eq!(resp.code, 200);

        let resp = test_auth(&admin_token_auth, &sc, admin_user.clone(), true);
        assert_eq!(resp.code, 401);
    }
}
