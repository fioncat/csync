use anyhow::Result;
use chrono::Local;
use rusqlite::{params, Connection, Transaction};

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

fn create_user_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}

fn check_user_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM user WHERE name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

fn insert_user(tx: &Transaction, name: &str, password: &str, salt: &str) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO user (name, password, salt, create_time, update_time) VALUES (?, ?, ?, ?, ?)",
        params![name, password, salt, now, now],
    )?;
    Ok(())
}

fn update_user(tx: &Transaction, name: &str, password: &str, salt: &str) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "UPDATE user SET password = ?, salt = ?, update_time = ? WHERE name = ?",
        params![password, salt, now, name],
    )?;
    Ok(())
}

fn check_role_exists(tx: &Transaction, name: &str) -> Result<bool> {
    let mut stmt = tx.prepare("SELECT COUNT(*) FROM role WHERE name = ?")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    Ok(count > 0)
}

fn delete_user_roles(tx: &Transaction, name: &str) -> Result<()> {
    tx.execute("DELETE FROM user_role WHERE user_name = ?", params![name])?;
    Ok(())
}

fn insert_user_role(tx: &Transaction, user_name: &str, role_name: &str) -> Result<()> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO user_role (user_name, role_name, create_time) VALUES (?, ?, ?)",
        params![user_name, role_name, now],
    )?;
    Ok(())
}
