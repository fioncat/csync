use std::fs::{self, File};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use console::style;
use csync_misc::config::PathSet;
use csync_misc::filelock::FileLock;
use daemonize::Daemonize as RawDaemonize;
use sysinfo::{Pid, ProcessesToUpdate, System};

use crate::DaemonizeAction;

pub struct Daemonize {
    out_path: PathBuf,
    logs_path: PathBuf,

    pid: Option<u32>,
}

impl Daemonize {
    pub fn new(ps: &PathSet) -> Result<Self> {
        let out_path = ps.data_dir.join("daemon.out");
        let logs_path = ps.data_dir.join("logs").join("daemon.log");

        let lock_path = ps.data_dir.join("daemon.lock");
        match FileLock::try_acquire(lock_path.clone())? {
            Some(_) => {
                // The daemon has stopped
                Ok(Self {
                    out_path,
                    logs_path,
                    pid: None,
                })
            }
            None => {
                // The daemon is running, try to get its pid
                let pid: u32 = fs::read_to_string(&lock_path)
                    .context("read daemon lock file")?
                    .parse()
                    .context("parse daemon pid")?;
                if !is_process_running(pid) {
                    bail!("The process in lock file is not running, please try again later");
                }
                Ok(Self {
                    out_path,
                    logs_path,
                    pid: Some(pid),
                })
            }
        }
    }

    pub fn handle(&self, action: DaemonizeAction) -> Result<bool> {
        match action {
            DaemonizeAction::Start => {
                if self.pid.is_some() {
                    bail!("The daemon is already running");
                }
                self.start()?;
                Ok(true)
            }
            DaemonizeAction::Stop => {
                if self.pid.is_none() {
                    bail!("The daemon is not running");
                }

                self.stop()?;
                Ok(false)
            }
            DaemonizeAction::Restart => {
                self.stop()?;
                self.start()?;
                Ok(true)
            }
            DaemonizeAction::Status => {
                let is_terminal = io::stdout().is_terminal();
                match self.pid {
                    Some(pid) => {
                        let hint = if is_terminal {
                            format!("{}", style("running").green().bold())
                        } else {
                            String::from("running")
                        };
                        println!("{hint}, {pid}");
                    }
                    None => {
                        let hint = if is_terminal {
                            format!("{}", style("stopped").red().bold())
                        } else {
                            String::from("stopped")
                        };
                        println!("{hint}");
                    }
                }
                Ok(false)
            }
            DaemonizeAction::Out => {
                redirect_file(&self.out_path)?;
                Ok(false)
            }
            DaemonizeAction::Logs => {
                redirect_file(&self.logs_path)?;
                Ok(false)
            }
        }
    }

    fn start(&self) -> Result<()> {
        println!("Starting daemon");
        let stdout = File::create(&self.out_path).context("create daemon out file")?;
        let stderr = stdout.try_clone()?;

        let daemonize = RawDaemonize::new().stdout(stdout).stderr(stderr);

        daemonize.start().context("start daemon")?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if let Some(pid) = self.pid {
            println!("Stopping daemon {pid}");
            kill_process(pid);

            thread::sleep(Duration::from_secs(1));
            if is_process_running(pid) {
                bail!("Daemon is still running after sending kill signal");
            }
        }
        Ok(())
    }
}

fn is_process_running(pid: u32) -> bool {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.process(Pid::from_u32(pid)).is_some()
}

fn kill_process(pid: u32) {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    if let Some(process) = sys.process(Pid::from_u32(pid)) {
        process.kill();
    }
}

fn redirect_file(path: &Path) -> Result<()> {
    match File::open(path) {
        Ok(file) => {
            io::copy(&mut &file, &mut io::stdout())?;
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("open file"),
    }
}
