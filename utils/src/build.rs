use std::env;
use std::error::Error;
use std::process::Command;

use simple_error::bail;
use vergen::EmitBuilder;

fn uncommitted_count() -> Result<usize, Box<dyn Error>> {
    let output = exec_git(&["status", "-s"])?;
    let lines = output.trim().split("\n");
    Ok(lines.filter(|line| !line.trim().is_empty()).count())
}

fn exec_git(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut cmd = Command::new("git");
    let output = cmd.args(args).output()?;
    if !output.status.success() {
        let cmd = format!("git {}", args.join(" "));
        bail!("Execute git command {} failed", cmd);
    }
    let output = String::from_utf8(output.stdout)?;
    Ok(output.trim().to_string())
}

pub fn run(cargo_version: &str) -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .rustc_semver()
        .rustc_llvm_version()
        .rustc_channel()
        .build_timestamp()
        .emit()?;

    let describe = match exec_git(&["describe", "--tags"]) {
        Ok(d) => d,
        Err(_) => String::from("unknown"),
    };
    let sha = exec_git(&["rev-parse", "HEAD"])?;
    let short_sha = exec_git(&["rev-parse", "--short", "HEAD"])?;

    let stable_tag = format!("v{cargo_version}");
    let (mut version, mut build_type) = if stable_tag == describe {
        if cargo_version.ends_with("alpha") {
            (cargo_version.to_string(), "alpha")
        } else if cargo_version.ends_with("beta") {
            (cargo_version.to_string(), "beta")
        } else if cargo_version.ends_with("rc") {
            (cargo_version.to_string(), "pre-release")
        } else {
            (cargo_version.to_string(), "stable")
        }
    } else {
        (format!("{cargo_version}-dev_{short_sha}"), "dev")
    };

    let uncommitted_count = uncommitted_count()?;
    if uncommitted_count > 0 {
        version = format!("{version}-uncommitted");
        build_type = "dev-uncommitted";
    }

    println!("cargo:rustc-env=BUILD_VERSION={version}");
    println!("cargo:rustc-env=BUILD_TYPE={build_type}");
    println!("cargo:rustc-env=BUILD_SHA={sha}");
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        env::var("TARGET").unwrap()
    );

    Ok(())
}

#[macro_export]
macro_rules! build_info {
    ($name:expr) => {
        println!("{} {}", $name, env!("BUILD_VERSION"));
        println!(
            "rustc {}-{}-{}",
            env!("VERGEN_RUSTC_SEMVER"),
            env!("VERGEN_RUSTC_LLVM_VERSION"),
            env!("VERGEN_RUSTC_CHANNEL")
        );
        println!();
        println!("Build type:   {}", env!("BUILD_TYPE"));
        println!("Build target: {}", env!("BUILD_TARGET"));
        println!("Commit SHA:   {}", env!("BUILD_SHA"));
        println!("Build time:   {}", env!("VERGEN_BUILD_TIMESTAMP"));
    };
}
