use crate::server::db::{Database, RoleRecord, UserRecord};
use crate::time::{advance_mock_time, current_timestamp};
use crate::types::user::RoleRule;

pub fn run_user_tests(db: &Database) {
    let users = [
        UserRecord {
            name: "Alice".to_string(),
            hash: "123".to_string(),
            salt: "456".to_string(),
            create_time: 0,
            update_time: 0,
        },
        UserRecord {
            name: "Bob".to_string(),
            hash: "789".to_string(),
            salt: "012".to_string(),
            create_time: 0,
            update_time: 0,
        },
        UserRecord {
            name: "Charlie".to_string(),
            hash: "345".to_string(),
            salt: "678".to_string(),
            create_time: 0,
            update_time: 0,
        },
        UserRecord {
            name: "David".to_string(),
            hash: "901".to_string(),
            salt: "234".to_string(),
            create_time: 0,
            update_time: 0,
        },
    ];

    db.with_transaction(|tx, _cache| {
        for user in users.iter() {
            tx.create_user(user).unwrap();
        }
        Ok(())
    })
    .unwrap();

    // Test get
    db.with_transaction(|tx, _cache| {
        let user = tx.get_user("Alice").unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.hash, "123");
        assert_eq!(user.salt, "456");
        let now = current_timestamp();
        assert_eq!(user.create_time, now);
        assert_eq!(user.update_time, now);

        assert!(tx.is_user_exists("Alice").unwrap());
        assert!(tx.is_user_exists("Bob").unwrap());
        assert!(tx.is_user_exists("Charlie").unwrap());
        assert!(tx.is_user_exists("David").unwrap());

        assert!(!tx.is_user_exists("Eve").unwrap());

        Ok(())
    })
    .unwrap();

    // Test list
    db.with_transaction(|tx, _cache| {
        let mut users_list = tx.list_users().unwrap();
        assert_eq!(users_list.len(), users.len());

        users_list.sort_by(|a, b| a.name.cmp(&b.name));
        for (i, user) in users_list.iter().enumerate() {
            assert_eq!(user.name, users[i].name);
            assert_eq!(user.hash, users[i].hash);
            assert_eq!(user.salt, users[i].salt);
        }

        Ok(())
    })
    .unwrap();

    // Test update
    db.with_transaction(|tx, _cache| {
        advance_mock_time(5);
        tx.update_user_password("Alice", "new_password", "new_salt")
            .unwrap();
        let user = tx.get_user("Alice").unwrap();
        assert_eq!(user.name, "Alice");
        assert_eq!(user.hash, "new_password");
        assert_eq!(user.salt, "new_salt");

        // The updated user should be at the top of the list
        let users_list = tx.list_users().unwrap();
        assert_eq!(users_list[0].name, "Alice");

        advance_mock_time(20);
        tx.update_user_time("Charlie").unwrap();
        let user = tx.get_user("Charlie").unwrap();
        assert_eq!(user.name, "Charlie");
        assert_eq!(user.update_time, current_timestamp());

        let users_list = tx.list_users().unwrap();
        assert_eq!(users_list[0].name, "Charlie");

        Ok(())
    })
    .unwrap();

    // Test delete
    db.with_transaction(|tx, _cache| {
        tx.delete_user("David").unwrap();
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        let users_list = tx.list_users().unwrap();
        assert_eq!(users_list.len(), users.len() - 1);
        assert!(!tx.is_user_exists("David").unwrap());
        Ok(())
    })
    .unwrap();

    // User cannot have same name
    let result = db.with_transaction(|tx, _cache| tx.create_user(&users[0]));
    assert!(result.is_err());

    // Get on non-existent user
    let result = db.with_transaction(|tx, _cache| tx.get_user("Eve"));
    assert!(result.is_err());

    // Update on non-existent user, should be OK
    let result = db
        .with_transaction(|tx, _cache| tx.update_user_password("Eve", "new_password", "new_salt"));
    assert!(result.is_ok());

    let result = db.with_transaction(|tx, _cache| tx.update_user_time("Eve"));
    assert!(result.is_ok());

    // Delete on non-existent user, should be OK
    let result = db.with_transaction(|tx, _cache| tx.delete_user("Eve"));
    assert!(result.is_ok());
}

pub fn run_role_tests(db: &Database) {
    let roles = [
        RoleRecord {
            name: "test".to_string(),
            rules: vec![RoleRule {
                resources: vec!["text".to_string()].into_iter().collect(),
                verbs: vec!["put".to_string(), "delete".to_string()]
                    .into_iter()
                    .collect(),
            }],
            create_time: 0,
            update_time: 0,
        },
        RoleRecord {
            name: "wheel".to_string(),
            rules: vec![RoleRule {
                resources: vec!["*".to_string()].into_iter().collect(),
                verbs: vec!["*".to_string()].into_iter().collect(),
            }],
            create_time: 0,
            update_time: 0,
        },
    ];

    // Test create roles
    db.with_transaction(|tx, _cache| {
        for role in roles.iter() {
            tx.create_role(role).unwrap();
        }
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let role = tx.get_role("test").unwrap();
        assert_eq!(role.name, "test");
        assert_eq!(role.rules, roles[0].rules);
        let now = current_timestamp();
        assert_eq!(role.create_time, now);
        assert_eq!(role.update_time, now);

        assert!(tx.is_role_exists("test").unwrap());
        assert!(tx.is_role_exists("wheel").unwrap());

        assert!(!tx.is_role_exists("nonexistent_role").unwrap());

        Ok(())
    })
    .unwrap();
}
