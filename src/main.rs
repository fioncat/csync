#![allow(dead_code)]

use log::{error, info, LevelFilter};

mod clipboard;
mod config;
mod hash;
mod logs;
mod net;
mod sync;

fn main() {
    logs::init(LevelFilter::Info).unwrap();

    info!("hello");
    error!("error");
}
