use std::path::PathBuf;

use clap::ValueEnum;
use daemonize::Daemonize as RawDaemonize;
use sysinfo::{Pid, System};

pub struct Daemonize {
    stdout_path: PathBuf,
    stderr_path: PathBuf,

    pid_path: PathBuf,

    pid: Option<u32>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DaemonizeAction {
    Start,
    Restart,
    Status,
    Stop,
    Stdout,
    Stderr,
    Logs,
}

fn is_process_running(pid: u32) -> bool {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.process(Pid::from_u32(pid)).is_some()
}
