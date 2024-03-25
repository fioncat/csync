use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use anyhow::bail;
use anyhow::{Context, Result};
use log::info;
use sha2::{Digest, Sha256};

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

    pub fn execute(&mut self) -> Result<Option<Vec<u8>>> {
        let mut child = match self.cmd.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                bail!(
                    "cannot find command `{}`, please make sure it is installed",
                    self.get_name()
                );
            }
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("cannot launch command `{}`", self.get_name()))
            }
        };

        if let Some(input) = &self.input {
            let handle = child.stdin.as_mut().unwrap();
            handle
                .write_all(input)
                .with_context(|| format!("write input to command `{}`", self.get_name()))?;
            drop(child.stdin.take());
        }

        let mut stdout = child.stdout.take();

        let status = child.wait().context("wait command done")?;
        let output = match stdout.as_mut() {
            Some(stdout) => {
                let mut out = Vec::new();
                stdout
                    .read_to_end(&mut out)
                    .with_context(|| format!("read stdout from command `{}`", self.get_name()))?;
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

    #[inline]
    fn get_name(&self) -> &str {
        self.cmd.get_program().to_str().unwrap_or("<unknown>")
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
