use csync_misc::api::user::{GetUserRequest, PatchUserRequest, PutUserRequest, User};
use csync_misc::api::QueryRequest;

use crate::db::types::{CreateUserParams, UserPassword};
use crate::db::Database;

pub fn run_user_tests(db: &Database) {
    test_create(db);
    test_get(db);
    test_update(db);
    test_delete(db);
}

fn test_create(db: &Database) {
    let users = [
        CreateUserParams {
            user: PutUserRequest {
                name: String::from("white"),
                password: String::from("test_password"),
                admin: true,
            },
            salt: String::from("test_salt"),
            update_time: 50,
        },
        CreateUserParams {
            user: PutUserRequest {
                name: String::from("black"),
                password: String::from("test123"),
                admin: false,
            },
            salt: String::from("test_salt_2"),
            update_time: 100,
        },
    ];

    db.with_transaction(|tx| {
        for user in users {
            tx.create_user(user)?;
        }
        Ok(())
    })
    .unwrap();
}

fn test_get(db: &Database) {
    let white_user = User {
        name: String::from("white"),
        admin: true,
        update_time: 50,
    };
    let black_user = User {
        name: String::from("black"),
        admin: false,
        update_time: 100,
    };

    db.with_transaction(|tx| {
        let users = tx.get_users(GetUserRequest::default())?;
        assert_eq!(users.len(), 2);
        assert_eq!(users[0], black_user);
        assert_eq!(users[1], white_user);

        let users = tx.get_users(GetUserRequest {
            query: QueryRequest {
                limit: Some(1),
                offset: Some(1),
                ..Default::default()
            },
            ..Default::default()
        })?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], white_user);

        let users = tx.get_users(GetUserRequest {
            name: Some(String::from("black")),
            ..Default::default()
        })?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], black_user);

        assert!(tx.has_user(String::from("white"))?);
        assert!(tx.has_user(String::from("black"))?);
        assert!(!tx.has_user(String::from("none"))?);

        let count = tx.count_users(GetUserRequest::default())?;
        assert_eq!(count, 2);

        let users = tx.get_users(GetUserRequest {
            query: QueryRequest {
                search: Some(String::from("wh")),
                ..Default::default()
            },
            ..Default::default()
        })?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], white_user);
        let count = tx.count_users(GetUserRequest {
            query: QueryRequest {
                search: Some(String::from("wh")),
                ..Default::default()
            },
            ..Default::default()
        })?;
        assert_eq!(count, 1);

        let up = tx.get_user_password(String::from("white"))?;
        assert_eq!(
            up,
            UserPassword {
                name: String::from("white"),
                password: String::from("test_password"),
                salt: String::from("test_salt"),
                admin: true,
            }
        );

        let up = tx.get_user_password(String::from("black"))?;
        assert_eq!(
            up,
            UserPassword {
                name: String::from("black"),
                password: String::from("test123"),
                salt: String::from("test_salt_2"),
                admin: false,
            }
        );

        let result = tx.get_user_password(String::from("none"));
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();
}

fn test_update(db: &Database) {
    db.with_transaction(|tx| {
        let expect = User {
            name: String::from("white"),
            admin: true,
            update_time: 4000,
        };

        tx.update_user(
            PatchUserRequest {
                name: String::from("white"),
                password: Some(String::from("new_password")),
                ..Default::default()
            },
            4000,
        )?;

        let users = tx.get_users(GetUserRequest::default())?;
        assert_eq!(users.len(), 2);
        assert_eq!(users[0], expect);

        let up = tx.get_user_password(String::from("white"))?;
        assert_eq!(
            up,
            UserPassword {
                name: String::from("white"),
                password: String::from("new_password"),
                salt: String::from("test_salt"),
                admin: true,
            }
        );

        let expect = User {
            name: String::from("black"),
            admin: true,
            update_time: 5000,
        };
        tx.update_user(
            PatchUserRequest {
                name: String::from("black"),
                admin: Some(true),
                ..Default::default()
            },
            5000,
        )?;

        let users = tx.get_users(GetUserRequest::default())?;
        assert_eq!(users.len(), 2);
        assert_eq!(users[0], expect);

        let up = tx.get_user_password(String::from("black"))?;
        assert_eq!(
            up,
            UserPassword {
                name: String::from("black"),
                password: String::from("test123"),
                salt: String::from("test_salt_2"),
                admin: true,
            }
        );

        Ok(())
    })
    .unwrap();
}

fn test_delete(db: &Database) {
    db.with_transaction(|tx| {
        tx.delete_user("white")?;
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx| {
        let users = tx.get_users(GetUserRequest::default())?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, String::from("black"));
        Ok(())
    })
    .unwrap();
}
