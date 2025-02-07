use anyhow::{bail, Context, Result};
use csync_misc::types::user::RoleRule;
use rusqlite::{params, Connection, Transaction};

use crate::db::{RoleRecord, UserRecord};
use crate::now::current_timestamp;

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS user (
    name TEXT PRIMARY KEY NOT NULL,
    hash TEXT NOT NULL,
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

/// Creates a new user with the given name, password and salt
pub fn create_user(tx: &Transaction, user: &UserRecord) -> Result<()> {
    let now = current_timestamp();
    tx.execute(
        "INSERT INTO user (name, hash, salt, create_time, update_time) VALUES (?, ?, ?, ?, ?)",
        params![user.name, user.hash, user.salt, now, now],
    )?;
    Ok(())
}

/// Gets the basic details of a user
pub fn get_user(tx: &Transaction, name: &str) -> Result<UserRecord> {
    let mut stmt =
        tx.prepare("SELECT name, hash, salt, create_time, update_time FROM user WHERE name = ?")?;
    let user = stmt.query_row([name], |row| {
        Ok(UserRecord {
            name: row.get(0)?,
            hash: row.get(1)?,
            salt: row.get(2)?,
            create_time: row.get(3)?,
            update_time: row.get(4)?,
        })
    })?;
    Ok(user)
}

/// Lists all users, ordered by update time descending
pub fn list_users(tx: &Transaction) -> Result<Vec<UserRecord>> {
    let mut stmt = tx.prepare(
        "SELECT name, hash, salt, create_time, update_time FROM user ORDER BY update_time DESC",
    )?;
    let users = stmt
        .query_map([], |row| {
            Ok(UserRecord {
                name: row.get(0)?,
                hash: row.get(1)?,
                salt: row.get(2)?,
                create_time: row.get(3)?,
                update_time: row.get(4)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();
    Ok(users)
}

/// Checks if a user with the given name exists
pub fn is_user_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM user WHERE name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

/// Updates the password and salt for an existing user
pub fn update_user_password(tx: &Transaction, name: &str, hash: &str, salt: &str) -> Result<()> {
    let now = current_timestamp();
    tx.execute(
        "UPDATE user SET hash = ?, salt = ?, update_time = ? WHERE name = ?",
        params![hash, salt, now, name],
    )?;
    Ok(())
}

/// Updates the last update time for a user
pub fn update_user_time(tx: &Transaction, name: &str) -> Result<()> {
    let now = current_timestamp();
    tx.execute(
        "UPDATE user SET update_time = ? WHERE name = ?",
        params![now, name],
    )?;
    Ok(())
}

/// Deletes a user from the database
pub fn delete_user(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM user WHERE name = ?", params![name])?;
    Ok(())
}

/// Assigns a role to a user
pub fn create_user_role(tx: &Transaction, name: &str, role: &str) -> Result<()> {
    let now = current_timestamp();
    tx.execute(
        "INSERT INTO user_role (user_name, role_name, create_time) VALUES (?, ?, ?)",
        params![name, role, now],
    )?;
    Ok(())
}

/// Deletes all role assignments for a user
pub fn delete_user_roles(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM user_role WHERE user_name = ?", params![name])?;
    Ok(())
}

/// Checks if a role is currently assigned to any users
pub fn is_role_in_use(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM user_role WHERE role_name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

const GET_USER_ROLES_SQL: &str = r#"
SELECT r.name, r.rules, r.create_time, r.update_time
FROM role AS r
LEFT JOIN user_role AS ur ON r.name = ur.role_name
WHERE ur.user_name = ?;
"#;

/// Get the roles assigned to a user
pub fn list_user_roles(tx: &Transaction, name: &str) -> Result<Vec<RoleRecord>> {
    let mut stmt = tx.prepare(GET_USER_ROLES_SQL)?;
    let roles = stmt
        .query_map([name], |row| {
            let raw_roles: String = row.get(1)?;
            let rules: Vec<RoleRule> = serde_json::from_str(&raw_roles).unwrap_or(vec![]);
            Ok(RoleRecord {
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

/// Creates a new role into the database
pub fn create_role(tx: &Transaction, role: &RoleRecord) -> Result<()> {
    let now = current_timestamp();
    let rules = serde_json::to_string(&role.rules).context("failed to serialize rules")?;
    tx.execute(
        "INSERT INTO role (name, rules, create_time, update_time) VALUES (?, ?, ?, ?)",
        params![role.name, rules, now, now],
    )?;
    Ok(())
}

/// Gets the details of a role
pub fn get_role(tx: &Transaction, name: &str) -> Result<RoleRecord> {
    let mut stmt = tx.prepare("SELECT rules, create_time, update_time FROM role WHERE name = ?")?;
    let role = stmt.query_row([name], |row| {
        let raw_rules: String = row.get(0)?;
        let rules: Vec<RoleRule> = serde_json::from_str(&raw_rules).unwrap_or(vec![]);
        Ok(RoleRecord {
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

/// Lists all roles, ordered by update time descending
pub fn list_roles(tx: &Transaction) -> Result<Vec<RoleRecord>> {
    let mut stmt = tx.prepare(
        "SELECT name, rules, create_time, update_time FROM role ORDER BY update_time DESC",
    )?;
    let roles = stmt
        .query_map([], |row| {
            let raw_rules: String = row.get(1)?;
            let rules: Vec<RoleRule> = serde_json::from_str(&raw_rules).unwrap_or(vec![]);
            Ok(RoleRecord {
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

/// Checks if a role with the given name exists
pub fn is_role_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM role WHERE name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

/// Updates the rules for an existing role
pub fn update_role_rules(tx: &Transaction, role: &str, rules: &[RoleRule]) -> Result<()> {
    let now = current_timestamp();
    let rules = serde_json::to_string(&rules).context("failed to serialize rules")?;
    tx.execute(
        "UPDATE role SET rules = ?, update_time = ? WHERE name = ?",
        params![rules, now, role],
    )?;
    Ok(())
}

/// Deletes a role from the database
pub fn delete_role(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM role WHERE name = ?", params![name])?;
    Ok(())
}
