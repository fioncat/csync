use std::process;

use clap::Parser;
use cmd::App;

mod clipboard;
mod cmd;
mod config;
mod hash;
mod logs;
mod net;
mod sync;

#[tokio::main]
async fn main() {
    let app = App::parse();
    if let Err(err) = app.run().await {
        eprintln!("Error: {:#}", err);
        process::exit(1);
    }
}
