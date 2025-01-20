mod file;
mod image;
mod text;
mod user;

use super::Database;

pub fn run_all_tests(db: &Database) {
    user::run_user_tests(db);
    user::run_role_tests(db);
}
