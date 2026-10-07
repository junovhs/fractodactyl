//! Drives the real `fd render --refine`: progressive phases (docs/spec/LOD.md) report
//! their work, every block ends final or fallback, accepted blocks skip supersamples,
//! and every computed sample equals the full render's.
use fd_samples::{Column, ColumnSet, Evidence, Reader};
use std::path::{Path, PathBuf};
use std::process::Command;

fn fd(args: &[&str]) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().lines().map(String::from).collect()
}

fn file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fd-refine-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn num(lines: &[String], name: &str) -> u64 {
    let v = lines.iter().find_map(|l| l.strip_prefix(&format!("{name} "))).unwrap_or_else(|| panic!("no {name} in {lines:?}"));
    v.parse().unwrap()
}

/// `(samples, iterations)` of `phase NAME blocks B samples S iterations I`.
fn phase(lines: &[String], name: &str) -> (u64, u64) {
    let l = lines.iter().find(|l| l.starts_with(&format!("phase {name} "))).unwrap();
    let w: Vec<&str> = l.split(' ').collect();
    (w[5].parse().unwrap(), w[7].parse().unwrap())
}

fn read(p: &Path) -> fd_samples::Samples {
    let mut r = Reader::open(p).unwrap();
    r.read(ColumnSet::of(&[Column::Nu, Column::De, Column::Bound])).unwrap()
}

const VIEW: [&str; 10] = ["--re", "0.5", "--im", "0", "--width", "1", "--size", "32x32", "--iter", "5000"];

fn render(kernel: &str, ss: &str, out: &Path, extra: &[&str]) -> Vec<String> {
    let o = out.to_str().unwrap();
    let mut a = vec!["render"];
    a.extend(VIEW);
    a.extend(["--ss", ss, "--kernel", kernel, "--columns", "nu,de,bound", "--threads", "2", "-o", o]);
    a.extend(extra);
    fd(&a)
}

#[test]
fn accepted_blocks_skip_supersamples() {
    for kernel in ["auto", "fx"] {
        let (full, prog) = (file(&format!("full-{kernel}.fds")), file(&format!("prog-{kernel}.fds")));
        render(kernel, "3", &full, &[]);
        let l = render(kernel, "3", &prog, &["--refine", "8"]);
        assert_eq!(num(&l, "full_samples"), 96 * 96);
        // Every block ends final or fallback, after preview and sparse.
        let blocks: Vec<&String> = l.iter().filter(|x| x.starts_with("block ")).collect();
        assert_eq!(blocks.len(), 16);
        assert_eq!(num(&l, "blocks.final") + num(&l, "blocks.fallback"), 16);
        for b in &blocks {
            let m: u8 = b.split(' ').nth(4).unwrap().parse().unwrap();
            assert_eq!(m & 3, 3, "{b}");
            assert!((m & 8 != 0) ^ (m & 16 != 0), "{b}");
            assert_eq!(b.ends_with(" final"), m & 8 != 0, "{b}");
        }
        let finals = num(&l, "blocks.final");
        assert!(finals > 0 && num(&l, "blocks.fallback") > 0, "{kernel}: {l:?}");
        // Work per phase adds up; accepted blocks skipped their other 8 of 9 samples.
        let (p, s, d) = (phase(&l, "preview"), phase(&l, "sparse"), phase(&l, "dense"));
        assert_eq!(p.0 + s.0 + d.0, num(&l, "samples"));
        assert_eq!(p.1 + s.1 + d.1, num(&l, "iterations"));
        assert_eq!(p.0 + s.0, 32 * 32);
        assert_eq!(num(&l, "skipped_samples"), finals * 64 * 8);
        // Computed samples equal the full render's; only Heuristic copies differ.
        let (a, b) = (read(&full), read(&prog));
        let (an, bn) = (a.nu.as_ref().unwrap(), b.nu.as_ref().unwrap());
        let differ = (0..a.class.len()).filter(|&k| a.class[k] != b.class[k] || an[k].to_bits() != bn[k].to_bits());
        let mut n = 0;
        for k in differ {
            assert_eq!(b.class[k].evidence(), Some(Evidence::Heuristic), "{kernel}: sample {k}");
            n += 1;
        }
        assert!(n > 0 && n as u64 <= num(&l, "skipped_samples"), "{kernel}: {n}");
        // The full render spends more iterations than the progressive one.
        let mut bench = vec!["bench"];
        bench.extend(VIEW);
        bench.extend(["--ss", "3", "--kernel", kernel, "--columns", "nu,de,bound", "--threads", "2", "--runs", "1"]);
        let json = fd(&bench).join("");
        let total: u64 = json.split("\"iterations\":{\"total\":").nth(1).unwrap().split(',').next().unwrap().parse().unwrap();
        assert!(total > num(&l, "iterations"), "{kernel}: full {total} vs {l:?}");
    }
}

#[test]
fn heuristic_kernel_falls_back_everywhere() {
    // pert-fx-scaled/1 writes no Bounded evidence: nothing is skipped, all blocks fall back.
    let l = render("scaled", "2", &file("scaled.fds"), &["--refine", "16"]);
    assert_eq!(num(&l, "blocks.final"), 0);
    assert_eq!(num(&l, "blocks.fallback"), 4);
    assert_eq!(num(&l, "skipped_samples"), 0);
    assert!(l.iter().filter(|x| x.starts_with("block ")).all(|x| x.ends_with(" mask 23 fallback")), "{l:?}");
}
