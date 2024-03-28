use std::fs;
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::bail;
use anyhow::{Context, Result};
use log::info;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::{self, Instant};

pub fn ensure_dir<P: AsRef<Path>>(dir: P) -> Result<()> {
    match fs::read_dir(dir.as_ref()) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(dir.as_ref())
                .with_context(|| format!("create directory '{}'", dir.as_ref().display()))?;
            Ok(())
        }
        Err(err) => {
            Err(err).with_context(|| format!("read directory '{}'", dir.as_ref().display()))
        }
    }
}

pub struct BuildInfo {
    version: &'static str,
    build_type: &'static str,
    build_target: &'static str,
    build_sha: &'static str,
    build_time: &'static str,
}

impl BuildInfo {
    #[inline]
    pub fn new() -> Self {
        Self {
            version: env!("CSYNC_VERSION"),
            build_type: env!("CSYNC_BUILD_TYPE"),
            build_target: env!("CSYNC_TARGET"),
            build_sha: env!("CSYNC_SHA"),
            build_time: env!("VERGEN_BUILD_TIMESTAMP"),
        }
    }

    pub fn log(&self) {
        info!(
            "Welcome to csync, version {} ({}), target '{}', commit '{}', build time '{}'",
            self.version, self.build_type, self.build_target, self.build_sha, self.build_time
        );
    }
}

pub struct Cmd {
    cmd: Command,

    input: Option<Vec<u8>>,
}

impl Cmd {
    pub fn new(args: &[String], input: Option<Vec<u8>>, capture_output: bool) -> Self {
        let name = &args[0];
        let args = &args[1..];

        let mut cmd = Command::new(name);
        cmd.stderr(Stdio::piped());
        if !capture_output {
            cmd.stdout(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::piped());
        }

        if input.is_some() {
            cmd.stdin(Stdio::piped());
        }

        if !args.is_empty() {
            cmd.args(args);
        }

        Cmd { cmd, input }
    }

    pub async fn execute(&mut self) -> Result<Option<Vec<u8>>> {
        let mut child = match self.cmd.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                bail!("cannot find command, please make sure it is installed");
            }
            Err(e) => return Err(e).context("cannot launch command"),
        };

        if let Some(input) = &self.input {
            let handle = child.stdin.as_mut().unwrap();
            handle
                .write_all(input)
                .await
                .context("write input to command")?;
            drop(child.stdin.take());
        }

        let mut stdout = child.stdout.take();

        let status =
            match time::timeout_at(Instant::now() + Duration::from_secs(1), child.wait()).await {
                Ok(result) => result.context("wait command exit")?,
                Err(_) => bail!("execute command timeout after 1s"),
            };
        let output = match stdout.as_mut() {
            Some(stdout) => {
                let mut out = Vec::new();
                stdout
                    .read_to_end(&mut out)
                    .await
                    .context("read stdout from command")?;
                Some(out)
            }
            None => None,
        };

        match status.code() {
            Some(code) => {
                if code != 0 {
                    bail!("command exited with bad code {code}");
                }
                Ok(output)
            }
            None => bail!("command exited with unknown code"),
        }
    }
}

pub fn get_digest(data: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(data);
    let result = hash.finalize();
    format!("{:x}", result)
}

pub fn shellexpand(s: impl AsRef<str>) -> Result<String> {
    shellexpand::full(s.as_ref())
        .with_context(|| format!("expand env for '{}'", s.as_ref()))
        .map(|s| s.into_owned())
}
