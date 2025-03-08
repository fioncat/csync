mod blob;
mod user;

use anyhow::{bail, Result};
use csync_misc::api::user::PutUserRequest;

use super::types::CreateUserParams;
use super::Database;

pub fn run_tests(db: &Database) {
    blob::run_blob_tests(db);
    user::run_user_tests(db);

    test_rollback(db);
}

fn test_rollback(db: &Database) {
    let result: Result<()> = db.with_transaction(|tx| {
        tx.create_user(CreateUserParams {
            user: PutUserRequest {
                name: String::from("none"),
                password: String::from("test123"),
                admin: true,
            },
            salt: String::from("test_salt"),
            update_time: 50,
        })
        .unwrap();

        bail!("rollback");
    });
    assert!(result.is_err());

    db.with_transaction(|tx| {
        assert!(!tx.has_user(String::from("none"))?);
        Ok(())
    })
    .unwrap();
}
