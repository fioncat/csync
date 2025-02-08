use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    csync_build::run()
}
