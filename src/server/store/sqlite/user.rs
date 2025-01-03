use anyhow::{bail, Context, Result};
use chrono::Local;
use rusqlite::{params, Connection, Transaction};

use crate::types::user::{Password, Role, RoleRule, User};

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS user (
    name TEXT PRIMARY KEY NOT NULL,
    password TEXT NOT NULL,
    salt TEXT NOT NULL,
    create_time INTEGER NOT NULL,
    update_time INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS role (
    name TEXT PRIMARY KEY NOT NULL,
    rules TEXT NOT NULL,
    create_time INTEGER NOT NULL,
    update_time INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_role (
    user_name TEXT NOT NULL,
    role_name TEXT NOT NULL,
    create_time INTEGER NOT NULL,
    PRIMARY KEY (user_name, role_name)
);

CREATE INDEX IF NOT EXISTS idx_user_role_user ON user_role(user_name);
CREATE INDEX IF NOT EXISTS idx_user_role_role ON user_role(role_name);
"#;

/// Creates the required tables for user management in the database
pub fn create_user_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}

/// Checks if a user with the given name exists
pub fn check_user_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM user WHERE name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

/// Inserts a new user with the given name, password and salt
pub fn insert_user(tx: &Transaction, name: &str, password: &str, salt: &str) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO user (name, password, salt, create_time, update_time) VALUES (?, ?, ?, ?, ?)",
        params![name, password, salt, now, now],
    )?;
    Ok(())
}

/// Updates the password and salt for an existing user
pub fn update_user_password(
    tx: &Transaction,
    name: &str,
    password: &str,
    salt: &str,
) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "UPDATE user SET password = ?, salt = ?, update_time = ? WHERE name = ?",
        params![password, salt, now, name],
    )?;
    Ok(())
}

/// Updates the last update time for a user
pub fn update_user_time(tx: &Transaction, name: &str) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "UPDATE user SET update_time = ? WHERE name = ?",
        params![now, name],
    )?;
    Ok(())
}

/// Checks if a role with the given name exists
pub fn check_role_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM role WHERE name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

/// Deletes all role assignments for a user
pub fn delete_user_roles(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM user_role WHERE user_name = ?", params![name])?;
    Ok(())
}

/// Assigns a role to a user
pub fn insert_user_role(tx: &Transaction, user_name: &str, role_name: &str) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO user_role (user_name, role_name, create_time) VALUES (?, ?, ?)",
        params![user_name, role_name, now],
    )?;
    Ok(())
}

/// Gets the basic details of a user
pub fn get_user_detail(tx: &Transaction, name: &str) -> Result<User> {
    let mut stmt = tx.prepare("SELECT name, create_time, update_time FROM user WHERE name = ?")?;
    let user = stmt.query_row([name], |row| {
        Ok(User {
            name: row.get(0)?,
            role_names: vec![],
            roles: None,
            password: None,
            create_time: row.get(1)?,
            update_time: row.get(2)?,
        })
    })?;
    Ok(user)
}

const GET_USER_ROLES_SQL: &str = r#"
SELECT r.name, r.rules, r.create_time, r.update_time
FROM role AS r
LEFT JOIN user_role AS ur ON r.name = ur.role_name
WHERE ur.user_name = ?;
"#;

/// Get the roles assigned to a user
pub fn get_user_roles(tx: &Transaction, name: &str) -> Result<Vec<Role>> {
    let mut stmt = tx.prepare(GET_USER_ROLES_SQL)?;
    let roles = stmt
        .query_map([name], |row| {
            let raw_roles: String = row.get(1)?;
            let rules: Vec<RoleRule> = serde_json::from_str(&raw_roles).unwrap_or(vec![]);
            Ok(Role {
                name: row.get(0)?,
                rules,
                create_time: row.get(2)?,
                update_time: row.get(3)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    for role in roles.iter() {
        if role.rules.is_empty() {
            bail!("role '{}' has no rules, you should delete it", role.name);
        }
    }

    Ok(roles)
}

/// Deletes a user from the database
pub fn delete_user(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM user WHERE name = ?", params![name])?;
    Ok(())
}

/// Lists all users, ordered by update time descending
pub fn list_users(tx: &Transaction) -> Result<Vec<User>> {
    let mut stmt =
        tx.prepare("SELECT name, create_time, update_time FROM user ORDER BY update_time DESC")?;
    let users = stmt
        .query_map([], |row| {
            Ok(User {
                name: row.get(0)?,
                role_names: vec![],
                roles: None,
                password: None,
                create_time: row.get(1)?,
                update_time: row.get(2)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    Ok(users)
}

/// Gets the list of role names assigned to a user
pub fn list_user_role_names(tx: &Transaction, user_name: &str) -> Result<Vec<String>> {
    let mut stmt = tx.prepare("SELECT role_name FROM user_role WHERE user_name = ?")?;
    let role_names = stmt
        .query_map([user_name], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<String>>();
    Ok(role_names)
}

/// Gets the password information for a user
pub fn get_user_password(tx: &Transaction, name: &str) -> Result<Password> {
    let mut stmt = tx.prepare("SELECT password, salt FROM user WHERE name = ?")?;
    let (hash, salt) = stmt.query_row([name], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(Password { hash, salt })
}

/// Inserts a new role into the database
pub fn insert_role(tx: &Transaction, role: &Role) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    let rules = serde_json::to_string(&role.rules).context("failed to serialize rules")?;
    tx.execute(
        "INSERT INTO role (name, rules, create_time, update_time) VALUES (?, ?, ?, ?)",
        params![role.name, rules, now, now],
    )?;
    Ok(())
}

/// Updates the rules for an existing role
pub fn update_role_rules(tx: &Transaction, role: &Role) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    let rules = serde_json::to_string(&role.rules).context("failed to serialize rules")?;
    tx.execute(
        "UPDATE role SET rules = ?, update_time = ? WHERE name = ?",
        params![rules, now, role.name],
    )?;
    Ok(())
}

/// Gets the details of a role
pub fn get_role(tx: &Transaction, name: &str) -> Result<Role> {
    let mut stmt = tx.prepare("SELECT rules, create_time, update_time FROM role WHERE name = ?")?;
    let role = stmt.query_row([name], |row| {
        let raw_rules: String = row.get(0)?;
        let rules: Vec<RoleRule> = serde_json::from_str(&raw_rules).unwrap_or(vec![]);
        Ok(Role {
            name: name.to_string(),
            rules,
            create_time: row.get(1)?,
            update_time: row.get(2)?,
        })
    })?;
    if role.rules.is_empty() {
        bail!("role '{}' has no rules, you should delete it", name);
    }
    Ok(role)
}

/// Checks if a role is currently assigned to any users
pub fn is_role_in_use(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM user_role WHERE role_name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

/// Deletes a role from the database
pub fn delete_role(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM role WHERE name = ?", params![name])?;
    Ok(())
}

/// Lists all roles, ordered by update time descending
pub fn list_roles(tx: &Transaction) -> Result<Vec<Role>> {
    let mut stmt = tx.prepare(
        "SELECT name, rules, create_time, update_time FROM role ORDER BY update_time DESC",
    )?;
    let roles = stmt
        .query_map([], |row| {
            let raw_rules: String = row.get(1)?;
            let rules: Vec<RoleRule> = serde_json::from_str(&raw_rules).unwrap_or(vec![]);
            Ok(Role {
                name: row.get(0)?,
                rules,
                create_time: row.get(2)?,
                update_time: row.get(3)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    Ok(roles)
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_user_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn test_check_user_exists() {
        let mut conn = setup_db();
        let tx = conn.transaction().unwrap();

        // Test when user does not exist
        assert!(!check_user_exists(&tx, "alice").unwrap());

        // Create test user
        insert_user(&tx, "alice", "hashed_password", "random_salt").unwrap();
        tx.commit().unwrap();

        // Create new transaction and test when user exists
        let tx = conn.transaction().unwrap();
        assert!(check_user_exists(&tx, "alice").unwrap());
    }

    #[test]
    fn test_insert_user() {
        let mut conn = setup_db();

        // Test data
        let name = "alice";
        let password = "hashed_password";
        let salt = "random_salt";

        // Insert user with a transaction
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, name, password, salt).unwrap();
            tx.commit().unwrap();
        }

        // Start new transaction for verification
        let tx = conn.transaction().unwrap();

        // Verify user exists
        assert!(check_user_exists(&tx, name).unwrap());

        // Verify password and salt
        let password_info = get_user_password(&tx, name).unwrap();
        assert_eq!(password_info.hash, password);
        assert_eq!(password_info.salt, salt);

        // Verify user details
        let user = get_user_detail(&tx, name).unwrap();
        assert_eq!(user.name, name);
        assert!(user.create_time > 0);
        assert_eq!(user.create_time, user.update_time);
        assert!(user.role_names.is_empty());
        assert!(user.roles.is_none());
        assert!(user.password.is_none());
    }

    #[test]
    fn test_update_user_password() {
        let mut conn = setup_db();

        // Test data
        let name = "alice";
        let password = "original_password";
        let salt = "original_salt";

        // First create a user
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, name, password, salt).unwrap();
            tx.commit().unwrap();
        }

        // Sleep to verify update time
        sleep(Duration::from_secs(2));

        // Update password
        let new_password = "new_password";
        let new_salt = "new_salt";
        {
            let tx = conn.transaction().unwrap();
            update_user_password(&tx, name, new_password, new_salt).unwrap();
            tx.commit().unwrap();
        }

        // Verify updated password
        let tx = conn.transaction().unwrap();

        // Check password info
        let password_info = get_user_password(&tx, name).unwrap();
        assert_eq!(password_info.hash, new_password);
        assert_eq!(password_info.salt, new_salt);

        // Verify update time changed
        let user = get_user_detail(&tx, name).unwrap();
        assert!(user.update_time > user.create_time);
    }

    #[test]
    fn test_update_user_time() {
        let mut conn = setup_db();

        // Test data
        let name = "alice";
        let password = "hashed_password";
        let salt = "random_salt";

        // First create a user
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, name, password, salt).unwrap();
            tx.commit().unwrap();
        }

        // Get original timestamps
        let tx = conn.transaction().unwrap();
        let original_user = get_user_detail(&tx, name).unwrap();
        let original_create_time = original_user.create_time;
        let original_update_time = original_user.update_time;
        tx.commit().unwrap();

        // Sleep to ensure timestamp will be different
        sleep(Duration::from_secs(2));

        // Update user time
        {
            let tx = conn.transaction().unwrap();
            update_user_time(&tx, name).unwrap();
            tx.commit().unwrap();
        }

        // Verify updated time
        let tx = conn.transaction().unwrap();
        let updated_user = get_user_detail(&tx, name).unwrap();

        // Create time should not change
        assert_eq!(updated_user.create_time, original_create_time);
        // Update time should be greater than original
        assert!(updated_user.update_time > original_update_time);
    }

    #[test]
    fn test_check_role_exists() {
        let mut conn = setup_db();

        // Test when role does not exist
        {
            let tx = conn.transaction().unwrap();
            assert!(!check_role_exists(&tx, "admin").unwrap());
            tx.commit().unwrap();
        }

        // Create a test role
        {
            let tx = conn.transaction().unwrap();
            let role = Role {
                name: "admin".to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            tx.commit().unwrap();
        }

        // Start new transaction for verification
        let tx = conn.transaction().unwrap();

        // Test when role exists
        assert!(check_role_exists(&tx, "admin").unwrap());

        // Test non-existent role again with different name
        assert!(!check_role_exists(&tx, "non_existent_role").unwrap());
    }

    #[test]
    fn test_delete_user_roles() {
        let mut conn = setup_db();

        // Test data
        let user_name = "alice";
        let role_names = ["admin", "user"];

        // Create user and roles first
        {
            let tx = conn.transaction().unwrap();

            // Create user
            insert_user(&tx, user_name, "password", "salt").unwrap();

            // Create roles
            for role_name in &role_names {
                let role = Role {
                    name: role_name.to_string(),
                    rules: vec![],
                    create_time: 0,
                    update_time: 0,
                };
                insert_role(&tx, &role).unwrap();
            }

            // Assign roles to user
            for role_name in &role_names {
                insert_user_role(&tx, user_name, role_name).unwrap();
            }

            tx.commit().unwrap();
        }

        // Verify roles were assigned
        {
            let tx = conn.transaction().unwrap();
            let assigned_roles = list_user_role_names(&tx, user_name).unwrap();
            assert_eq!(assigned_roles.len(), 2);
            assert!(assigned_roles.contains(&"admin".to_string()));
            assert!(assigned_roles.contains(&"user".to_string()));
            tx.commit().unwrap();
        }

        // Delete user roles
        {
            let tx = conn.transaction().unwrap();
            delete_user_roles(&tx, user_name).unwrap();
            tx.commit().unwrap();
        }

        // Verify roles were deleted
        let tx = conn.transaction().unwrap();
        let remaining_roles = list_user_role_names(&tx, user_name).unwrap();
        assert!(remaining_roles.is_empty());
    }

    #[test]
    fn test_insert_user_role() {
        let mut conn = setup_db();

        // Test data
        let user_name = "alice";
        let role_name = "admin";

        // Create user and role first
        {
            let tx = conn.transaction().unwrap();

            // Create user
            insert_user(&tx, user_name, "password", "salt").unwrap();

            // Create role
            let role = Role {
                name: role_name.to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();

            tx.commit().unwrap();
        }

        // Insert user role
        {
            let tx = conn.transaction().unwrap();
            insert_user_role(&tx, user_name, role_name).unwrap();
            tx.commit().unwrap();
        }

        // Verify role was assigned
        let tx = conn.transaction().unwrap();
        let assigned_roles = list_user_role_names(&tx, user_name).unwrap();
        assert_eq!(assigned_roles.len(), 1);
        assert_eq!(assigned_roles[0], role_name);
        drop(tx);

        // Test inserting the same role again (should fail)
        {
            let tx = conn.transaction().unwrap();
            assert!(insert_user_role(&tx, user_name, role_name).is_err());
        }
    }

    #[test]
    fn test_get_user_detail() {
        let mut conn = setup_db();

        // Test data
        let name = "alice";
        let password = "hashed_password";
        let salt = "random_salt";

        // Create user first
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, name, password, salt).unwrap();
            tx.commit().unwrap();
        }

        // Test getting user details
        {
            let tx = conn.transaction().unwrap();

            // Get user details
            let user = get_user_detail(&tx, name).unwrap();

            // Verify user details
            assert_eq!(user.name, name);
            assert!(user.create_time > 0);
            assert_eq!(user.create_time, user.update_time);
            assert!(user.role_names.is_empty());
            assert!(user.roles.is_none());
            assert!(user.password.is_none());

            tx.commit().unwrap();
        }

        // Test getting non-existent user
        {
            let tx = conn.transaction().unwrap();
            assert!(get_user_detail(&tx, "non_existent_user").is_err());
        }

        // Add roles to user and verify details again
        {
            let tx = conn.transaction().unwrap();

            // Create and assign a role
            let role = Role {
                name: "admin".to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            insert_user_role(&tx, name, "admin").unwrap();

            tx.commit().unwrap();
        }

        // Verify user details with role
        let tx = conn.transaction().unwrap();
        let user = get_user_detail(&tx, name).unwrap();
        assert_eq!(user.name, name);
        assert!(user.create_time > 0);
        assert_eq!(user.create_time, user.update_time);
        assert!(user.role_names.is_empty()); // role_names is populated by other functions
        assert!(user.roles.is_none());
        assert!(user.password.is_none());
    }

    #[test]
    fn test_get_user_roles() {
        let mut conn = setup_db();

        // Test data
        let user_name = "alice";
        let role_names = ["admin", "user"];

        let role_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["put".to_string(), "get".to_string(), "delete".to_string()],
        };

        // Create user and roles first
        {
            let tx = conn.transaction().unwrap();

            // Create user
            insert_user(&tx, user_name, "password", "salt").unwrap();

            // Create roles with rules
            for role_name in &role_names {
                let role = Role {
                    name: role_name.to_string(),
                    rules: vec![role_rule.clone()], // Need at least one rule
                    create_time: 0,
                    update_time: 0,
                };
                insert_role(&tx, &role).unwrap();
            }

            // Assign roles to user
            for role_name in &role_names {
                insert_user_role(&tx, user_name, role_name).unwrap();
            }

            tx.commit().unwrap();
        }

        // Test getting user roles
        {
            let tx = conn.transaction().unwrap();

            let roles = get_user_roles(&tx, user_name).unwrap();

            // Verify roles count
            assert_eq!(roles.len(), 2);

            // Verify each role
            for role in roles {
                assert!(role_names.contains(&role.name.as_str()));
                assert_eq!(role.rules, vec![role_rule.clone()]);
                assert!(role.create_time > 0);
                assert_eq!(role.create_time, role.update_time);
            }

            tx.commit().unwrap();
        }

        // Test getting roles for non-existent user
        {
            let tx = conn.transaction().unwrap();
            let roles = get_user_roles(&tx, "non_existent_user").unwrap();
            assert!(roles.is_empty());
        }

        // Test getting roles for user with no roles
        {
            let tx = conn.transaction().unwrap();

            // Create new user without roles
            insert_user(&tx, "bob", "password", "salt").unwrap();
            tx.commit().unwrap();

            let tx = conn.transaction().unwrap();
            let roles = get_user_roles(&tx, "bob").unwrap();
            assert!(roles.is_empty());
        }

        // Test role with empty rules (should fail)
        {
            let tx = conn.transaction().unwrap();

            // Create role with empty rules
            let role = Role {
                name: "empty_role".to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            insert_user_role(&tx, user_name, "empty_role").unwrap();

            tx.commit().unwrap();

            let tx = conn.transaction().unwrap();
            assert!(get_user_roles(&tx, user_name).is_err());
        }
    }

    #[test]
    fn test_delete_user() {
        let mut conn = setup_db();

        // Test data
        let user_name = "alice";
        let password = "password";
        let salt = "salt";

        // Create user
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, user_name, password, salt).unwrap();
            tx.commit().unwrap();
        }

        // Verify user exists
        {
            let tx = conn.transaction().unwrap();
            assert!(check_user_exists(&tx, user_name).unwrap());
        }

        // Delete user
        {
            let tx = conn.transaction().unwrap();
            delete_user(&tx, user_name).unwrap();
            tx.commit().unwrap();
        }

        // Verify user is deleted
        {
            let tx = conn.transaction().unwrap();

            // User should not exist
            assert!(!check_user_exists(&tx, user_name).unwrap());

            // Getting user details should fail
            assert!(get_user_detail(&tx, user_name).is_err());

            // Getting user password should fail
            assert!(get_user_password(&tx, user_name).is_err());
        }

        // Test deleting non-existent user (should succeed)
        {
            let tx = conn.transaction().unwrap();
            assert!(delete_user(&tx, "non_existent_user").is_ok());
        }
    }

    #[test]
    fn test_list_users() {
        let mut conn = setup_db();

        // Test empty list
        {
            let tx = conn.transaction().unwrap();
            let users = list_users(&tx).unwrap();
            assert!(users.is_empty());
        }

        // Create multiple users
        {
            let tx = conn.transaction().unwrap();

            // Create first user
            insert_user(&tx, "alice", "password1", "salt1").unwrap();

            // Sleep to ensure different timestamps
            sleep(Duration::from_secs(1));

            // Create second user
            insert_user(&tx, "bob", "password2", "salt2").unwrap();

            // Sleep to ensure different timestamps
            sleep(Duration::from_secs(1));

            // Create third user
            insert_user(&tx, "charlie", "password3", "salt3").unwrap();

            tx.commit().unwrap();
        }

        // Verify initial order (most recent first)
        {
            let tx = conn.transaction().unwrap();
            let users = list_users(&tx).unwrap();

            assert_eq!(users.len(), 3);
            assert_eq!(users[0].name, "charlie");
            assert_eq!(users[1].name, "bob");
            assert_eq!(users[2].name, "alice");

            // Verify timestamps are in descending order
            assert!(users[0].update_time > users[1].update_time);
            assert!(users[1].update_time > users[2].update_time);
        }

        // Update a user's time and verify new order
        {
            sleep(Duration::from_secs(1));

            let tx = conn.transaction().unwrap();
            update_user_time(&tx, "alice").unwrap();
            tx.commit().unwrap();
        }

        // Verify updated order
        {
            let tx = conn.transaction().unwrap();
            let users = list_users(&tx).unwrap();

            assert_eq!(users.len(), 3);
            assert_eq!(users[0].name, "alice"); // Now first due to update
            assert_eq!(users[1].name, "charlie");
            assert_eq!(users[2].name, "bob");

            // Verify timestamps are still in descending order
            assert!(users[0].update_time > users[1].update_time);
            assert!(users[1].update_time > users[2].update_time);
        }
    }

    #[test]
    fn test_list_user_role_names() {
        let mut conn = setup_db();

        // Test data
        let user_name = "alice";
        let role_names = ["admin", "user", "guest"];

        // Create user first
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, user_name, "password", "salt").unwrap();
            tx.commit().unwrap();
        }

        // Test empty role list for new user
        {
            let tx = conn.transaction().unwrap();
            let roles = list_user_role_names(&tx, user_name).unwrap();
            assert!(roles.is_empty());
        }

        // Create roles and assign them one by one
        {
            let tx = conn.transaction().unwrap();

            // Create roles
            for role_name in &role_names {
                let role = Role {
                    name: role_name.to_string(),
                    rules: vec![],
                    create_time: 0,
                    update_time: 0,
                };
                insert_role(&tx, &role).unwrap();
            }

            tx.commit().unwrap();
        }

        // Assign roles one by one and verify list
        for (i, &role_name) in role_names.iter().enumerate() {
            // Assign role
            {
                let tx = conn.transaction().unwrap();
                insert_user_role(&tx, user_name, role_name).unwrap();
                tx.commit().unwrap();
            }

            // Verify role list
            {
                let tx = conn.transaction().unwrap();
                let roles = list_user_role_names(&tx, user_name).unwrap();
                assert_eq!(roles.len(), i + 1);
                assert!(roles.contains(&role_name.to_string()));
            }
        }

        // Verify final complete list
        {
            let tx = conn.transaction().unwrap();
            let roles = list_user_role_names(&tx, user_name).unwrap();
            assert_eq!(roles.len(), role_names.len());
            for role_name in &role_names {
                assert!(roles.contains(&role_name.to_string()));
            }
        }

        // Test list for non-existent user
        {
            let tx = conn.transaction().unwrap();
            let roles = list_user_role_names(&tx, "non_existent_user").unwrap();
            assert!(roles.is_empty());
        }
    }

    #[test]
    fn test_get_user_password() {
        let mut conn = setup_db();

        // Test data
        let name = "alice";
        let password = "hashed_password";
        let salt = "random_salt";

        // Test getting password for non-existent user
        {
            let tx = conn.transaction().unwrap();
            assert!(get_user_password(&tx, name).is_err());
        }

        // Create user
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, name, password, salt).unwrap();
            tx.commit().unwrap();
        }

        // Test getting password for existing user
        {
            let tx = conn.transaction().unwrap();
            let password_info = get_user_password(&tx, name).unwrap();

            // Verify password info
            assert_eq!(password_info.hash, password);
            assert_eq!(password_info.salt, salt);
        }

        // Update password and verify
        {
            let tx = conn.transaction().unwrap();
            let new_password = "new_password";
            let new_salt = "new_salt";
            update_user_password(&tx, name, new_password, new_salt).unwrap();
            tx.commit().unwrap();

            let tx = conn.transaction().unwrap();
            let password_info = get_user_password(&tx, name).unwrap();

            // Verify updated password info
            assert_eq!(password_info.hash, new_password);
            assert_eq!(password_info.salt, new_salt);
        }
    }

    #[test]
    fn test_insert_role() {
        let mut conn = setup_db();

        // Test data
        let role_name = "admin";
        let role_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string(), "write".to_string()],
        };

        let role = Role {
            name: role_name.to_string(),
            rules: vec![role_rule.clone()],
            create_time: 0,
            update_time: 0,
        };

        // Test inserting role
        {
            let tx = conn.transaction().unwrap();
            insert_role(&tx, &role).unwrap();
            tx.commit().unwrap();
        }

        // Verify role exists and details are correct
        {
            let tx = conn.transaction().unwrap();

            // Check role exists
            assert!(check_role_exists(&tx, role_name).unwrap());

            // Get and verify role details
            let saved_role = get_role(&tx, role_name).unwrap();
            assert_eq!(saved_role.name, role_name);
            assert_eq!(saved_role.rules, vec![role_rule]);
            assert!(saved_role.create_time > 0);
            assert_eq!(saved_role.create_time, saved_role.update_time);
        }

        // Test inserting duplicate role (should fail)
        {
            let tx = conn.transaction().unwrap();
            assert!(insert_role(&tx, &role).is_err());
        }

        // Test inserting role with empty rules (should succeed as validation is in get_role)
        {
            let tx = conn.transaction().unwrap();
            let empty_role = Role {
                name: "empty_role".to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            assert!(insert_role(&tx, &empty_role).is_ok());
            tx.commit().unwrap();

            // Verify getting role with empty rules fails
            let tx = conn.transaction().unwrap();
            assert!(get_role(&tx, "empty_role").is_err());
        }
    }

    #[test]
    fn test_update_role_rules() {
        let mut conn = setup_db();

        // Test data
        let role_name = "admin";
        let initial_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string()],
        };

        // Create role first
        {
            let tx = conn.transaction().unwrap();
            let role = Role {
                name: role_name.to_string(),
                rules: vec![initial_rule.clone()],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            tx.commit().unwrap();
        }

        // Verify initial role
        {
            let tx = conn.transaction().unwrap();
            let role = get_role(&tx, role_name).unwrap();
            assert_eq!(role.rules, vec![initial_rule.clone()]);
            let initial_update_time = role.update_time;

            // Sleep to ensure timestamp will be different
            sleep(Duration::from_secs(1));

            // Update role rules
            let new_rule = RoleRule {
                objects: vec!["documents/*".to_string()],
                verbs: vec!["read".to_string(), "write".to_string()],
            };
            let updated_role = Role {
                name: role_name.to_string(),
                rules: vec![new_rule.clone()],
                create_time: role.create_time,
                update_time: role.update_time,
            };
            drop(tx);

            let tx = conn.transaction().unwrap();
            update_role_rules(&tx, &updated_role).unwrap();
            tx.commit().unwrap();

            // Verify updated role
            let tx = conn.transaction().unwrap();
            let role = get_role(&tx, role_name).unwrap();
            assert_eq!(role.rules, vec![new_rule]);
            assert!(role.update_time > initial_update_time);
            assert_eq!(role.create_time, initial_update_time);
        }

        // Test updating with empty rules (should succeed as validation is in get_role)
        {
            let tx = conn.transaction().unwrap();
            let empty_rules_role = Role {
                name: role_name.to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            assert!(update_role_rules(&tx, &empty_rules_role).is_ok());
            tx.commit().unwrap();

            // Verify getting role with empty rules fails
            let tx = conn.transaction().unwrap();
            assert!(get_role(&tx, role_name).is_err());
        }
    }

    #[test]
    fn test_get_role() {
        let mut conn = setup_db();

        // Test data
        let role_name = "admin";
        let role_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string(), "write".to_string()],
        };

        // Test getting non-existent role
        {
            let tx = conn.transaction().unwrap();
            assert!(get_role(&tx, role_name).is_err());
        }

        // Create role
        {
            let tx = conn.transaction().unwrap();
            let role = Role {
                name: role_name.to_string(),
                rules: vec![role_rule.clone()],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            tx.commit().unwrap();
        }

        // Test getting role
        {
            let tx = conn.transaction().unwrap();
            let role = get_role(&tx, role_name).unwrap();

            // Verify role details
            assert_eq!(role.name, role_name);
            assert_eq!(role.rules, vec![role_rule]);
            assert!(role.create_time > 0);
            assert_eq!(role.create_time, role.update_time);
        }

        // Test getting role with empty rules
        {
            let tx = conn.transaction().unwrap();

            // Update role to have empty rules
            let empty_role = Role {
                name: role_name.to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            update_role_rules(&tx, &empty_role).unwrap();
            tx.commit().unwrap();

            // Verify getting role with empty rules fails
            let tx = conn.transaction().unwrap();
            let err = get_role(&tx, role_name).unwrap_err();
            assert!(err.to_string().contains("no rules"));
        }
    }

    #[test]
    fn test_is_role_in_use() {
        let mut conn = setup_db();

        // Test data
        let role_name = "admin";
        let user_name = "alice";

        // Test non-existent role
        {
            let tx = conn.transaction().unwrap();
            assert!(!is_role_in_use(&tx, role_name).unwrap());
        }

        // Create role
        {
            let tx = conn.transaction().unwrap();
            let role = Role {
                name: role_name.to_string(),
                rules: vec![],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            tx.commit().unwrap();
        }

        // Test unused role
        {
            let tx = conn.transaction().unwrap();
            assert!(!is_role_in_use(&tx, role_name).unwrap());
        }

        // Create user and assign role
        {
            let tx = conn.transaction().unwrap();
            insert_user(&tx, user_name, "password", "salt").unwrap();
            insert_user_role(&tx, user_name, role_name).unwrap();
            tx.commit().unwrap();
        }

        // Test role in use
        {
            let tx = conn.transaction().unwrap();
            assert!(is_role_in_use(&tx, role_name).unwrap());
        }

        // Delete user-role assignment
        {
            let tx = conn.transaction().unwrap();
            delete_user_roles(&tx, user_name).unwrap();
            tx.commit().unwrap();
        }

        // Test role no longer in use
        {
            let tx = conn.transaction().unwrap();
            assert!(!is_role_in_use(&tx, role_name).unwrap());
        }
    }

    #[test]
    fn test_delete_role() {
        let mut conn = setup_db();

        // Test data
        let role_name = "admin";
        let role_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string(), "write".to_string()],
        };

        // Create role first
        {
            let tx = conn.transaction().unwrap();
            let role = Role {
                name: role_name.to_string(),
                rules: vec![role_rule],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role).unwrap();
            tx.commit().unwrap();
        }

        // Verify role exists
        {
            let tx = conn.transaction().unwrap();
            assert!(check_role_exists(&tx, role_name).unwrap());
        }

        // Delete role
        {
            let tx = conn.transaction().unwrap();
            delete_role(&tx, role_name).unwrap();
            tx.commit().unwrap();
        }

        // Verify role is deleted
        {
            let tx = conn.transaction().unwrap();

            // Role should not exist
            assert!(!check_role_exists(&tx, role_name).unwrap());

            // Getting role details should fail
            assert!(get_role(&tx, role_name).is_err());
        }

        // Test deleting non-existent role (should succeed)
        {
            let tx = conn.transaction().unwrap();
            assert!(delete_role(&tx, "non_existent_role").is_ok());
        }
    }

    #[test]
    fn test_list_roles() {
        let mut conn = setup_db();

        // Test empty list
        {
            let tx = conn.transaction().unwrap();
            let roles = list_roles(&tx).unwrap();
            assert!(roles.is_empty());
        }

        // Test data
        let role_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string(), "write".to_string()],
        };

        // Create multiple roles
        {
            let tx = conn.transaction().unwrap();

            // Create first role
            let role1 = Role {
                name: "role1".to_string(),
                rules: vec![role_rule.clone()],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role1).unwrap();

            // Sleep to ensure different timestamps
            sleep(Duration::from_secs(1));

            // Create second role
            let role2 = Role {
                name: "role2".to_string(),
                rules: vec![role_rule.clone()],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role2).unwrap();

            // Sleep to ensure different timestamps
            sleep(Duration::from_secs(1));

            // Create third role
            let role3 = Role {
                name: "role3".to_string(),
                rules: vec![role_rule.clone()],
                create_time: 0,
                update_time: 0,
            };
            insert_role(&tx, &role3).unwrap();

            tx.commit().unwrap();
        }

        // Verify initial order (most recent first)
        {
            let tx = conn.transaction().unwrap();
            let roles = list_roles(&tx).unwrap();

            assert_eq!(roles.len(), 3);
            assert_eq!(roles[0].name, "role3");
            assert_eq!(roles[1].name, "role2");
            assert_eq!(roles[2].name, "role1");

            // Verify timestamps are in descending order
            assert!(roles[0].update_time > roles[1].update_time);
            assert!(roles[1].update_time > roles[2].update_time);

            // Verify rules are preserved
            for role in &roles {
                assert_eq!(role.rules, vec![role_rule.clone()]);
            }
        }

        // Update a role's rules and verify new order
        {
            sleep(Duration::from_secs(1));

            let tx = conn.transaction().unwrap();
            let updated_role = Role {
                name: "role1".to_string(),
                rules: vec![role_rule.clone()],
                create_time: 0,
                update_time: 0,
            };
            update_role_rules(&tx, &updated_role).unwrap();
            tx.commit().unwrap();
        }

        // Verify updated order
        {
            let tx = conn.transaction().unwrap();
            let roles = list_roles(&tx).unwrap();

            assert_eq!(roles.len(), 3);
            assert_eq!(roles[0].name, "role1"); // Now first due to update
            assert_eq!(roles[1].name, "role3");
            assert_eq!(roles[2].name, "role2");

            // Verify timestamps are still in descending order
            assert!(roles[0].update_time > roles[1].update_time);
            assert!(roles[1].update_time > roles[2].update_time);
        }
    }
}
