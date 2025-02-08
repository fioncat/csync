use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tauri_build::build();
    csync_build::run()
}
