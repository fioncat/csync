use anyhow::Result;
use csync_misc::api::user::{GetUserRequest, PatchUserRequest, User};
use csync_misc::api::Value;
use log::debug;
use rusqlite::types::Value as DbValue;
use rusqlite::{params, params_from_iter, Connection, Transaction};

use crate::db::sql::{Select, Update};
use crate::db::types::{CreateUserParams, UserPassword};

use super::convert_values;

const CREATE_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS user (
    name TEXT PRIMARY KEY NOT NULL,
    admin INTEGER NOT NULL,
    password TEXT NOT NULL,
    salt TEXT NOT NULL,
    update_time INTEGER NOT NULL
);
"#;

pub fn create_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLE_SQL)?;
    Ok(())
}

pub fn create(tx: &Transaction, params: CreateUserParams) -> Result<()> {
    let sql = r#"
    INSERT INTO user (name, admin, password, salt, update_time)
    VALUES (?, ?, ?, ?, ?)
    "#;
    debug!("Database create_user: {sql}, {params:?}");
    tx.execute(
        sql,
        params![
            params.user.name,
            params.user.admin,
            params.user.password,
            params.salt,
            params.update_time,
        ],
    )?;

    Ok(())
}

pub fn update(tx: &Transaction, patch: PatchUserRequest, update_time: u64) -> Result<()> {
    let mut update = Update::new("user");

    if let Some(password) = patch.password {
        update.add_field("password", Value::Text(password));
    }

    if let Some(admin) = patch.admin {
        update.add_field("admin", Value::Bool(admin));
    }

    update.add_field("update_time", Value::Integer(update_time));

    update.add_where("name = ?", Value::Text(patch.name));

    let (sql, values) = update.build();
    if sql.is_empty() {
        return Ok(());
    }
    let values = convert_values(values);

    debug!("Database update_user: {sql}, {values:?}");
    tx.execute(&sql, params_from_iter(values.iter()))?;

    Ok(())
}

pub fn delete(tx: &Transaction, name: &str) -> Result<()> {
    let sql = "DELETE FROM user WHERE name = ?";
    debug!("Database delete_user: {sql}, {name}");
    tx.execute(sql, params![name])?;
    Ok(())
}

pub fn has(tx: &Transaction, name: String) -> Result<bool> {
    debug!("Database has_user: {name}");
    let req = GetUserRequest {
        name: Some(name),
        ..Default::default()
    };
    let count = count_users(tx, req)?;
    Ok(count > 0)
}

pub fn get_user_password(tx: &Transaction, name: String) -> Result<UserPassword> {
    let mut select = Select::new(vec!["name", "password", "salt", "admin"], "user");
    select.add_where("name = ?", Value::Text(name));

    let (sql, values) = select.build();
    let values = convert_values(values);

    debug!("Database get_user_password: {sql}, {values:?}");
    let mut stmt = tx.prepare(&sql)?;
    let up = stmt.query_row(params_from_iter(values), |row| {
        Ok(UserPassword {
            name: row.get(0)?,
            password: row.get(1)?,
            salt: row.get(2)?,
            admin: row.get(3)?,
        })
    })?;

    Ok(up)
}

pub fn count_users(tx: &Transaction, req: GetUserRequest) -> Result<u64> {
    let (sql, values) = build_select_sql(true, req);
    debug!("Database count_users: {sql}, {values:?}");

    let mut stmt = tx.prepare(&sql)?;

    let count: i64 = stmt.query_row(params_from_iter(values.iter()), |row| row.get(0))?;

    Ok(count as u64)
}

pub fn get_users(tx: &Transaction, req: GetUserRequest) -> Result<Vec<User>> {
    let (sql, values) = build_select_sql(false, req);
    debug!("Database get_users: {sql}, {values:?}");

    let mut stmt = tx.prepare(&sql)?;

    let users = stmt
        .query_map(params_from_iter(values), |row| {
            Ok(User {
                name: row.get(0)?,
                admin: row.get(1)?,
                update_time: row.get(2)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    Ok(users)
}

fn build_select_sql(count: bool, req: GetUserRequest) -> (String, Vec<DbValue>) {
    let mut select = if count {
        Select::count("user")
    } else {
        Select::new(vec!["name", "admin", "update_time"], "user")
    };

    if let Some(name) = req.name {
        select.add_where("name = ?", Value::Text(name));
    }

    select.set_query(req.query, "name");

    select.add_order_by("update_time DESC");

    let (sql, values) = select.build();
    let values = convert_values(values);

    (sql, values)
}
