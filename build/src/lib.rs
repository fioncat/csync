use std::env;
use std::error::Error;
use std::process::Command;

use simple_error::bail;
use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder, SysinfoBuilder};

fn uncommitted_count() -> usize {
    let output = match _exec_git(&["status", "-s"]) {
        Ok(output) => output,
        Err(_) => return 0,
    };
    let lines = output.trim().split('\n');
    lines.filter(|line| !line.trim().is_empty()).count()
}

fn exec_git(args: &[&str]) -> String {
    _exec_git(args).unwrap_or(String::from("unknown"))
}

fn _exec_git(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut cmd = Command::new("git");
    let output = cmd.args(args).output()?;
    if !output.status.success() {
        let cmd = format!("git {}", args.join(" "));
        bail!("Execute git command {} failed", cmd);
    }
    let output = String::from_utf8(output.stdout)?;
    Ok(output.trim().to_string())
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let build = BuildBuilder::all_build()?;
    let cargo = CargoBuilder::all_cargo()?;
    let rustc = RustcBuilder::all_rustc()?;
    let si = SysinfoBuilder::all_sysinfo()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .add_instructions(&si)?
        .emit()?;

    let mut version = exec_git(&["describe", "--tags"]);
    if uncommitted_count() > 0 {
        version = format!("{}-dirty", version);
    }

    println!("cargo:rustc-env=CSYNC_VERSION={version}");
    println!(
        "cargo:rustc-env=CSYNC_TARGET={}",
        env::var("TARGET").unwrap()
    );

    Ok(())
}
