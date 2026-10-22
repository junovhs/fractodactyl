//! Ordered native equivalent of the former scripts/check.sh, with bounded workers.
use std::ffi::OsString;
use std::process::{Command, ExitStatus};

fn code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        128 + status.signal().unwrap_or(1)
    }
    #[cfg(not(unix))]
    {
        1
    }
}

/// Run the existing required Cargo workflow; inherit output and return child failure.
pub fn run(args: &[OsString]) -> Result<i32, String> {
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        println!("usage: fd-dev check [--jobs N]\nJobs: explicit --jobs, otherwise JOBS, otherwise 2. Positive worker count required.\nRuns fmt, workspace all-target check, strict Clippy, workspace tests in order.");
        return Ok(0);
    }
    let raw = match args {
        [] => std::env::var_os("JOBS").unwrap_or_else(|| "2".into()),
        [flag, count] if flag == "--jobs" => count.clone(),
        _ => return Err("usage: fd-dev check [--jobs N]".into()),
    };
    let jobs = raw
        .to_str()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .ok_or("check jobs: expected a positive integer (--jobs, JOBS, default 2)")?
        .to_string();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    for args in [
        vec!["fmt", "--all", "--check"],
        vec!["check", "--workspace", "--all-targets", "-j", &jobs],
        vec![
            "clippy",
            "--workspace",
            "--all-targets",
            "-j",
            &jobs,
            "--",
            "-D",
            "warnings",
        ],
        vec!["test", "--workspace", "-j", &jobs],
    ] {
        eprintln!("fd-dev check: cargo {}", args.join(" "));
        let status = Command::new(&cargo)
            .args(&args)
            .env("RUST_TEST_THREADS", &jobs)
            .env("CARGO_BUILD_JOBS", &jobs)
            .status()
            .map_err(|e| format!("could not run cargo {}: {e}", args[0]))?;
        if !status.success() {
            return Ok(code(status));
        }
    }
    Ok(0)
}
