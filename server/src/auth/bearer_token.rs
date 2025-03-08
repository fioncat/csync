use anyhow::{bail, Result};
use chrono::Utc;
use csync_misc::api::user::User;

use crate::context::ServerContext;

pub fn auth_bearer_token(sc: &ServerContext, auth: String, is_remote: bool) -> Result<User> {
    let now = Utc::now().timestamp() as u64;
    let user = sc.jwt_validator.validate_token(&auth, now)?;

    if user.name == "admin" && is_remote {
        bail!("cannot auth as admin from remote");
    }

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_bearer_token() {
        let sc = ServerContext::new_test();

        let admin_user = User {
            name: String::from("test_admin"),
            admin: true,
            update_time: 0,
        };
        let user = User {
            name: String::from("test_user"),
            admin: false,
            update_time: 0,
        };

        let now = Utc::now().timestamp() as u64;
        let admin_token = sc
            .jwt_generator
            .generate_token(admin_user.clone(), now)
            .unwrap();
        let user_token = sc.jwt_generator.generate_token(user.clone(), now).unwrap();

        let result = auth_bearer_token(&sc, admin_token.token, true).unwrap();
        assert_eq!(result, admin_user);

        let result = auth_bearer_token(&sc, user_token.token, true).unwrap();
        assert_eq!(result, user);

        let result = auth_bearer_token(&sc, String::from("invalid token"), true);
        assert!(result.is_err());

        let admin_token = sc
            .jwt_generator
            .generate_token(
                User {
                    name: String::from("admin"),
                    admin: true,
                    update_time: 0,
                },
                now,
            )
            .unwrap();
        let result = auth_bearer_token(&sc, admin_token.token.clone(), true);
        assert!(result.is_err());

        let user = auth_bearer_token(&sc, admin_token.token, false).unwrap();
        assert_eq!(user.name, "admin");
        assert!(user.admin);
    }
}
