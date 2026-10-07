//! Drives the real `fd addr` binary: locate at 1e-1000 depth, then show/sample/owner
//! must round-trip exactly through their printed text.
use std::process::Command;

fn fd(args: &[&str]) -> Vec<(String, String)> {
    let out = Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap();
    assert!(out.status.success(), "fd {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| {
            let (k, v) = l.split_once(' ').unwrap();
            (k.to_string(), v.to_string())
        })
        .collect()
}

fn get<'a>(lines: &'a [(String, String)], name: &str) -> &'a str {
    &lines.iter().find(|(k, _)| k == name).unwrap_or_else(|| panic!("no {name}")).1
}

#[test]
fn shallow_values() {
    let s = fd(&["addr", "show", "3/1/6"]);
    assert_eq!((get(&s, "re"), get(&s, "im"), get(&s, "side")), ("-1.25", "-1.25", "0.5"));
    assert_eq!(get(&s, "parent"), "2/0/3");
    assert_eq!(get(&s, "children"), "4/2/c 4/3/c 4/2/d 4/3/d");
    assert_eq!(get(&fd(&["addr", "locate", "--re", "-1.25", "--im", "-1.25", "--level", "3"]), "key"), "3/1/6");
    assert_eq!(get(&fd(&["addr", "show", "0/0/0"]), "parent"), "-");
}

#[test]
fn extreme_depth_round_trip() {
    // A 1e-1000 view is ~2^-3322 wide; level 3340 tiles are ~2^-3338.
    let (re, im) = ("-1.7400623825793399052208441670658651", "0.0281944643415506992591050878373");
    let key = get(&fd(&["addr", "locate", "--re", re, "--im", im, "--level", "3340"]), "key").to_string();
    assert!(key.starts_with("3340/"));
    let s = fd(&["addr", "show", &key]);
    assert_eq!(get(&s, "key"), key);
    assert_eq!(get(&s, "side").len(), 2 + 3338);
    // The printed exact centre locates back to the same tile.
    let back = fd(&["addr", "locate", "--re", get(&s, "re"), "--im", get(&s, "im"), "--level", "3340"]);
    assert_eq!(get(&back, "key"), key);
    // Parent's children include this tile.
    let p = fd(&["addr", "show", get(&s, "parent")]);
    assert!(get(&p, "children").split(' ').any(|c| c == key));
    // Sample (i, j) of a 128x128 grid -> cell -> owner gives the same tile and indices.
    let cell = fd(&["addr", "sample", &key, "--grid", "7", "--at", "127,5"]);
    let owner = fd(&["addr", "owner", get(&cell, "key"), "--grid", "7"]);
    assert_eq!((get(&owner, "key"), get(&owner, "at")), (key.as_str(), "127,5"));
    let back = fd(&["addr", "locate", "--re", get(&cell, "re"), "--im", get(&cell, "im"), "--level", "3347"]);
    assert_eq!(get(&back, "key"), get(&cell, "key"));
}

#[test]
fn rejects_bad_input() {
    for args in [
        &["addr", "show", "1/2/0"][..],
        &["addr", "locate", "--re", "2", "--im", "0", "--level", "4"],
        &["addr", "sample", "1/0/0", "--grid", "2", "--at", "4,0"],
        &["addr", "owner", "1/0/0", "--grid", "2"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}
