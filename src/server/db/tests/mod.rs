mod file;
mod image;
mod text;
mod user;

use super::Database;

pub fn run_all_tests(db: &Database) {
    user::run_user_tests(db);
    user::run_role_tests(db);
    user::run_user_role_tests(db);

    text::run_text_tests(db);

    image::run_image_tests(db);
}
