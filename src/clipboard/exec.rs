use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

pub fn check_command(name: &str, args: &[&str]) -> Result<()> {
    if execute_read_command(name, args).is_err() {
        bail!("It had error to execute clipboard command '{name}', please check it is ready");
    }
    Ok(())
}

pub fn execute_write_command(name: &str, args: &[&str], data: &[u8]) -> Result<()> {
    let mut cmd = Command::new(name);

    if !args.is_empty() {
        cmd.args(args);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::piped());

    let mut child = cmd.spawn().context("launch clipboard copy command")?;

    let stdin = child.stdin.as_mut().unwrap();
    if let Err(err) = stdin.write_all(data) {
        return Err(err).context("write data to clipboard copy command");
    }
    drop(child.stdin.take());

    let status = child.wait().context("wait clipboard copy command done")?;
    if !status.success() {
        let code = status
            .code()
            .map(|code| code.to_string())
            .unwrap_or("<unknown>".to_string());
        bail!("clipboard copy command exited with bad code {code}");
    }

    Ok(())
}

pub fn execute_read_command(name: &str, args: &[&str]) -> Result<Vec<u8>> {
    let mut cmd = Command::new(name);

    if !args.is_empty() {
        cmd.args(args);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::piped());

    let output = cmd.output().context("execute clipboard paste command")?;

    if !output.status.success() {
        let code = output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or("<unknown>".to_string());
        if code == "1" {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim() == "Nothing is copied" {
                return Ok(Vec::new());
            }
        }
        bail!("clipboard paste command exited with bad code {code}");
    }

    Ok(output.stdout)
}
