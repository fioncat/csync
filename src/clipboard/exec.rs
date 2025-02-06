use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

/// Checks if a clipboard command is available and working
///
/// # Arguments
/// * `name` - Name of the command to check (e.g., "pbcopy", "wl-copy")
/// * `args` - Command line arguments for version/help check
///
/// # Returns
/// - `Ok(())` if command exists and works
/// - `Err` if command is not found or not working
pub fn check_command(name: &str, args: &[&str]) -> Result<()> {
    if execute_read_command(name, args).is_err() {
        bail!("It had error to execute clipboard command '{name}', please check it is ready");
    }
    Ok(())
}

/// Executes a command to write data to clipboard
///
/// # Arguments
/// * `name` - Name of the clipboard write command (e.g., "pbcopy", "wl-copy")
/// * `args` - Optional command line arguments
/// * `data` - Data to write to clipboard
///
/// # Returns
/// - `Ok(())` if write succeeds
/// - `Err` if command fails or writing fails
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

/// Executes a command to read data from clipboard
///
/// # Arguments
/// * `name` - Name of the clipboard read command (e.g., "pbpaste", "wl-paste")
/// * `args` - Optional command line arguments
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the clipboard data if read succeeds
/// - `Ok(Vec::new())` if clipboard is empty (special case for some commands)
/// - `Err` if command fails or reading fails
///
/// # Special Cases
/// Handles the special case where some clipboard commands return exit code 1
/// with "Nothing is copied" message to indicate empty clipboard
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
