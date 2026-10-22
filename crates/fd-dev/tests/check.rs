//! Drive actual Cargo workspaces on native platforms; no mock compiler or shell.
use std::path::Path;
use std::process::{Command, Output};

fn invoke(dir: &Path, args: &[&str], jobs: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fd-dev"));
    cmd.arg("check")
        .args(args)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", dir.join("target"));
    match jobs {
        Some(jobs) => {
            cmd.env("JOBS", jobs);
        }
        None => {
            cmd.env_remove("JOBS");
        }
    }
    cmd.output().unwrap()
}

#[test]
fn native_check_runs_real_cargo_with_bounds_and_fail_fast() {
    let base = std::env::temp_dir().join(format!("fd-dev-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let cases=[
        ("explicit", "pub fn marker() {}\n",None,vec!["--jobs","2"],Some("9"),2,0,4),
        ("environment","pub fn marker() {}\n",None,vec![],Some("1"),1,0,4),
        ("default","pub fn marker() {}\n",None,vec![],None,2,0,4),
        ("fmt-fail","pub fn marker( ){}\n",None,vec!["--jobs","2"],None,2,1,1),
        ("check-fail","pub fn marker() {\n    missing_symbol();\n}\n",None,vec!["--jobs","2"],None,2,101,2),
        ("clippy-fail","pub fn marker() -> u32 {\n    return 1;\n}\n",None,vec!["--jobs","2"],None,2,101,3),
        ("test-fail","pub fn marker() {}\n",Some("#[test]\nfn intentional_failure() {\n    panic!(\"intentional test failure\");\n}\n"),vec!["--jobs","2"],None,2,101,4),
    ];
    for (name, source, test, args, jobs, wanted, exit, stages) in cases {
        let dir = base.join(name);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("Cargo.toml"),"[package]\nname = \"check-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n").unwrap();
        std::fs::write(dir.join("src/lib.rs"), source).unwrap();
        std::fs::write(dir.join("build.rs"),format!("fn main() {{\n    assert_eq!(std::env::var(\"NUM_JOBS\").unwrap(), \"{wanted}\");\n    assert_eq!(std::env::var(\"RUST_TEST_THREADS\").unwrap(), \"{wanted}\");\n}}\n")).unwrap();
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        std::fs::write(dir.join("tests/workers.rs"),test.map(str::to_string).unwrap_or_else(||format!("#[test]\nfn worker_budget() {{\n    assert_eq!(std::env::var(\"RUST_TEST_THREADS\").unwrap(), \"{wanted}\");\n}}\n"))).unwrap();
        let output = invoke(&dir, &args, jobs);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(exit),
            "{name}: stdout={} stderr={stderr}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(
            stderr
                .lines()
                .filter(|l| l.starts_with("fd-dev check: cargo "))
                .count(),
            stages,
            "{name}: {stderr}"
        );
        if name == "check-fail" {
            assert!(stderr.contains("missing_symbol"));
        }
        if name == "clippy-fail" {
            assert!(stderr.contains("unneeded `return`"));
        }
        if name == "test-fail" {
            assert!(String::from_utf8_lossy(&output.stdout).contains("intentional test failure"));
        }
    }
    for (args, jobs) in [
        (vec!["--jobs", "0"], None),
        (vec!["--jobs", "nope"], None),
        (vec![], Some("0")),
    ] {
        let output = invoke(&base.join("default"), &args, jobs);
        assert_eq!(output.status.code(), Some(2));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("fd-dev check: cargo "));
    }
    std::fs::remove_dir_all(base).unwrap();
}
