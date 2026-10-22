//! Actual developer-command parity against captured real inputs and old Python output.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/report")
}
fn run(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd-dev"))
        .args(["bench", "report"])
        .arg(dir)
        .output()
        .unwrap()
}
fn same(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Number(a), Value::Number(b)) => assert_eq!(
            a.as_f64(),
            b.as_f64(),
            "numeric mismatch at {path}: {a} vs {b}"
        ),
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.len(), b.len(), "keys at {path}");
            for (k, v) in a {
                same(
                    v,
                    b.get(k).unwrap_or_else(|| panic!("missing {path}.{k}")),
                    &format!("{path}.{k}"),
                );
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "array at {path}");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(x, y, &format!("{path}[{i}]"));
            }
        }
        _ => assert_eq!(expected, actual, "mismatch at {path}"),
    }
}
#[test]
fn report_matches_captured_python_output() {
    let output = run(&fixture());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: Value =
        serde_json::from_slice(&std::fs::read(fixture().join("expected.json")).unwrap()).unwrap();
    same(&expected, &actual, "$");
    assert_eq!(actual["depth_bins"].as_array().unwrap().len(), 2);
}
#[test]
fn report_rejects_missing_or_malformed_inputs_cleanly() {
    for (name, file, replacement, message) in [
        ("missing", "A.jsonl", None, "A.jsonl"),
        ("bad-json", "B.jsonl", Some("{bad\n"), "B.jsonl"),
        (
            "missing-totals",
            "A.jsonl",
            Some("{\"record\":\"frame\"}\n"),
            "totals",
        ),
        (
            "missing-wall",
            "C.compile.time",
            Some("User time (seconds): 0.0\n"),
            "wall_seconds",
        ),
    ] {
        let dir = std::env::temp_dir().join(format!("fd-dev-report-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for input in std::fs::read_dir(fixture()).unwrap() {
            let input = input.unwrap();
            if input.file_type().unwrap().is_file() {
                std::fs::copy(input.path(), dir.join(input.file_name())).unwrap();
            }
        }
        match replacement {
            Some(text) => std::fs::write(dir.join(file), text).unwrap(),
            None => std::fs::remove_file(dir.join(file)).unwrap(),
        }
        let result = run(&dir);
        assert!(!result.status.success(), "{name} unexpectedly passed");
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains(message), "{name}: {error}");
        assert!(!error.contains("panicked"));
        assert!(result.stdout.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
