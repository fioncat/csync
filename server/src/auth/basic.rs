use anyhow::{bail, Context, Result};
use csync_misc::api::user::User;
use csync_misc::code;
use log::error;

use crate::context::ServerContext;

pub fn auth_basic(sc: &ServerContext, auth: String, is_remote: bool) -> Result<User> {
    let fields = auth.split(':').collect::<Vec<&str>>();
    if fields.len() != 2 {
        bail!("basic auth missing password");
    }

    let username = fields[0];
    let password = fields[1];
    let password = code::base64_decode_string(password).context("decode password base64")?;

    if username == "admin" {
        if is_remote {
            bail!("cannot login as admin from remote");
        }

        if password == sc.cfg.admin_password {
            return Ok(User {
                name: String::from("admin"),
                admin: true,
                update_time: 0,
            });
        }

        bail!("incorrect admin password");
    }

    let result = sc.db.with_transaction(|tx| {
        if !tx.has_user(username.to_string())? {
            return Ok(None);
        }

        let up = tx.get_user_password(username.to_string())?;
        let password = code::sha256(format!("{password}{}", up.salt));

        if password != up.password {
            return Ok(None);
        }

        Ok(Some(User {
            name: up.name,
            admin: up.admin,
            update_time: 0,
        }))
    });
    let user = match result {
        Ok(user) => user,
        Err(e) => {
            error!("Auth database error: {e:#}");
            bail!("database error");
        }
    };
    match user {
        Some(user) => Ok(user),
        None => bail!("incorrect username or password"),
    }
}

#[cfg(test)]
mod tests {
    use csync_misc::api::user::PutUserRequest;

    use crate::db::types::CreateUserParams;

    use super::*;

    #[test]
    fn test_auth_basic() {
        let sc = ServerContext::new_test();
        sc.db
            .with_transaction(|tx| {
                tx.create_user(CreateUserParams {
                    user: PutUserRequest {
                        name: String::from("test_admin"),
                        password: code::sha256("test123test_salt1"), // test123
                        admin: true,
                    },
                    salt: String::from("test_salt1"),
                    update_time: 50,
                })?;
                tx.create_user(CreateUserParams {
                    user: PutUserRequest {
                        name: String::from("test_normal"),
                        password: code::sha256("test222test_salt2"), // test222
                        admin: false,
                    },
                    salt: String::from("test_salt2"),
                    update_time: 50,
                })?;
                Ok(())
            })
            .unwrap();

        let expect_admin = User {
            name: String::from("test_admin"),
            admin: true,
            update_time: 0,
        };
        let expect_normal = User {
            name: String::from("test_normal"),
            admin: false,
            update_time: 0,
        };

        let auth = format!("test_admin:{}", code::base64_encode("test123"));
        let user = auth_basic(&sc, auth, true).unwrap();
        assert_eq!(user, expect_admin);

        let auth = format!("test_normal:{}", code::base64_encode("test222"));
        let user = auth_basic(&sc, auth, true).unwrap();
        assert_eq!(user, expect_normal);

        let auth = format!("test_normal:{}", code::base64_encode("xxx"));
        assert!(auth_basic(&sc, auth, true).is_err());

        let auth = format!("test_admin:{}", code::base64_encode(""));
        assert!(auth_basic(&sc, auth, true).is_err());

        let auth = format!("none:{}", code::base64_encode("test123"));
        assert!(auth_basic(&sc, auth, true).is_err());

        let auth = format!("admin:{}", code::base64_encode("admin_password123"));
        assert!(auth_basic(&sc, auth, true).is_err());

        let auth = format!("admin:{}", code::base64_encode("test123"));
        assert!(auth_basic(&sc, auth, false).is_err());

        let auth = format!("admin:{}", code::base64_encode("admin_password123"));
        let user = auth_basic(&sc, auth, false).unwrap();
        assert_eq!(
            user,
            User {
                name: String::from("admin"),
                admin: true,
                update_time: 0,
            }
        );
    }
}
