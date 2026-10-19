use std::process::Command;

fn inspect(text: &str, name: &str) -> std::process::Output {
    let path = std::env::temp_dir().join(format!("fd-style-{}-{name}.style", std::process::id()));
    std::fs::write(&path, text).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_fd"))
        .args(["style", "inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    result
}

#[test]
fn style_inspect_compiles_shared_reachable_work_and_rejects_invalid_graphs() {
    let graph="version 1\nsurface a\nemission b\nx = color #102030\ny = color #102030\na = add x y\nb = add x y\ndead = nonsense missing\n";
    let a = inspect(graph, "first");
    let b = inspect(graph, "second");
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
    assert_eq!(a.stdout, b.stdout);
    let report = String::from_utf8(a.stdout).unwrap();
    assert!(
        report.contains("reachable_nodes 4 unique_operations 2 pointwise_passes 1"),
        "{report}"
    );
    for (name, bad, expected) in [
        (
            "type",
            "version 1\nsurface a\nemission a\na = scalar 2\n",
            "must be colors",
        ),
        (
            "cycle",
            "version 1\nsurface a\nemission a\na = add b b\nb = add a a\n",
            "cycle",
        ),
        (
            "missing",
            "version 1\nsurface absent\nemission absent\n",
            "missing node",
        ),
        (
            "mixed",
            "version 1\nsurface a\nemission a\nx = color #000000\ny = vector 1 2\na = mul x y\n",
            "invalid inputs",
        ),
    ] {
        let out = inspect(bad, name);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains(expected));
    }
}
