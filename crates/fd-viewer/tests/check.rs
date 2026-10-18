//! VIEW-06: `fd-viewer --check` starts the real renderer (`fd explore`) and fetches the
//! viewer page and one render through it, without opening a window.
use std::process::Command;

#[test]
fn viewer_check_reaches_the_renderer() {
    let viewer = std::path::Path::new(env!("CARGO_BIN_EXE_fd-viewer"));
    let fd = viewer.with_file_name(if cfg!(windows) { "fd.exe" } else { "fd" });
    if !fd.is_file() {
        eprintln!(
            "skipped: no fd binary next to {} (build fd-cli)",
            viewer.display()
        );
        return;
    }
    let out = Command::new(viewer).arg("--check").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    eprintln!("{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());
    assert!(stdout.contains("64x36 render 6912 bytes"), "{stdout}");
}

/// SITE-11 (DEC-31): `fd-viewer --help` points at the site's manual, without a window.
#[test]
fn help_links_docs() {
    let out = Command::new(env!("CARGO_BIN_EXE_fd-viewer"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("https://fractodactyl.dev/docs/"), "{text}");
}
