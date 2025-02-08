mod file;
mod image;
mod text;
mod user;

use anyhow::{bail, Result};

use crate::db::UserRecord;

use super::Database;

pub fn run_all_db_tests(db: &Database) {
    user::run_user_tests(db);
    user::run_role_tests(db);
    user::run_user_role_tests(db);

    text::run_text_tests(db);

    image::run_image_tests(db);

    file::run_file_tests(db);

    test_rollback(db);
}

fn test_rollback(db: &Database) {
    let mut image_id = 0;
    let mut file_id = 0;
    let result: Result<()> = db.with_transaction(|tx, _cache| {
        tx.create_user(&UserRecord {
            name: "should_be_rollbacked".to_string(),
            hash: "123".to_string(),
            salt: "456".to_string(),
            create_time: 0,
            update_time: 0,
        })
        .unwrap();
        assert!(tx.is_user_exists("should_be_rollbacked").unwrap());

        let image = tx
            .create_image(image::mock_image("should_be_rollbacked"))
            .unwrap();
        image_id = image.id;
        assert!(tx.is_image_exists(image_id, None).unwrap());

        let file = tx
            .create_file(file::mock_file("should_be_rollbacked", "content", 0o644))
            .unwrap();
        file_id = file.id;
        assert!(tx.is_file_exists(file_id, None).unwrap());

        bail!("rollback");
    });
    assert!(result.is_err());

    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_user_exists("should_be_rollbacked").unwrap());
        assert!(!tx.is_image_exists(image_id, None).unwrap());
        assert!(!tx.is_file_exists(file_id, None).unwrap());
        Ok(())
    })
    .unwrap();
}
