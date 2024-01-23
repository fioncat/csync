use std::error::Error;

use csync_utils::build;

fn main() -> Result<(), Box<dyn Error>> {
    build::run(env!("CARGO_PKG_VERSION"))
}
