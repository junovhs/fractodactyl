//! Drives the real `fd orbit bla` and `fd render --bla`: a BLA table built over a stored
//! deep orbit is stored once (a second build deduplicates), renders frames at its width
//! and deeper with fewer iterations and the same escape counts as plain perturbation,
//! reports its fallback, and is refused where its contract does not hold.
use fd_samples::{Column, ColumnSet, Kind, Reader};
use std::path::Path;
use std::process::{Command, Output};

const RE: &str = "-0.743643887037158704752191506114774";
const IM: &str = "0.131825904205311970493132056385139";

fn fd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fd")).args(args).output().unwrap()
}

fn ok(args: &[&str]) -> String {
    let out = fd(args);
    assert!(out.status.success(), "fd {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn get(text: &str, k: &str) -> String {
    text.lines().find_map(|l| l.strip_prefix(&format!("{k} "))).unwrap_or_else(|| panic!("no {k}: {text}")).to_string()
}

fn view(width: &str) -> Vec<&str> {
    vec!["--re", RE, "--im", IM, "--width", width, "--size", "48x27", "--iter", "50000", "--columns", "nu,de", "--threads", "2"]
}

fn samples(p: &Path) -> fd_samples::Samples {
    Reader::open(p).unwrap().read(ColumnSet::of(&[Column::Class, Column::Nu, Column::De])).unwrap()
}

#[test]
fn stored_bla_skips_iterations_within_contract() {
    let dir = std::env::temp_dir().join(format!("fd-bla-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let st = dir.join("atlas");
    let st = st.to_str().unwrap();
    let orbit = get(&ok(&[&["orbit", "put", "--store", st][..], &view("1e-28")[..]].concat()), "orbit");
    let build = [&["orbit", "bla", "--store", st, "--orbit", orbit.as_str()][..], &view("1e-20")[..]].concat();
    let put = ok(&build);
    assert_eq!(get(&put, "orbit"), orbit);
    assert!(get(&put, "orbit_slab_bytes").parse::<u64>().unwrap() > 0, "{put}");
    assert!(get(&put, "valid_blocks").parse::<u64>().unwrap() > 0, "{put}");
    assert_eq!(get(&put, "result"), "stored");
    let again = ok(&build);
    assert_eq!(get(&again, "result"), "deduplicated");
    let bla = get(&put, "bla");

    // The table serves its own width and every deeper frame at the centre.
    for width in ["1e-20", "1e-28"] {
        let plain = dir.join(format!("plain-{width}.fds"));
        ok(&[&["render"][..], &view(width)[..], &["-o", plain.to_str().unwrap()]].concat());
        let fast = dir.join(format!("bla-{width}.fds"));
        let text = ok(&[&["render"][..], &view(width)[..], &["--store", st, "--bla", bla.as_str(), "-o", fast.to_str().unwrap()]].concat());
        assert!(text.contains("bla/1"), "{text}");
        let (its, eq): (u64, u64) = (get(&text, "iterations").parse().unwrap(), get(&text, "iterations.equivalent").parse().unwrap());
        assert!(get(&text, "bla.skipped").parse::<u64>().unwrap() > 0 && its < eq, "{text}");
        let fb: f64 = get(&text, "fallback.iteration_fraction").parse().unwrap();
        assert!((0.0..1.0).contains(&fb), "{text}");
        // First-order shift estimate of the dropped terms, in output pixels.
        let shift: f64 = get(&text, "bla.shift_px.max").parse().unwrap();
        assert!(shift > 0.0 && shift < 0.25, "{text}");
        let (a, b) = (samples(&plain), samples(&fast));
        let (an, bn, de) = (a.nu.unwrap(), b.nu.unwrap(), a.de.unwrap());
        let mut escaped = 0;
        for i in 0..a.class.len() {
            let esc = a.class[i].kind() == Some(Kind::Escaped);
            assert_eq!(esc, b.class[i].kind() == Some(Kind::Escaped), "{width} sample {i}");
            if esc {
                escaped += 1;
                // Equivalent displacement in output pixels (the oracle's measure). The
                // estimate is first order and leaves out f64 rounding, which both paths
                // add at about U / eps = 1/8 of the eps term: a factor 2 covers both.
                let px = (an[i] - bn[i]).abs() * f64::from(de[i]) * std::f64::consts::LN_2 / 2.0;
                assert!(px <= 2.0 * shift, "{width} sample {i}: nu {} vs {}, {px} px > 2 x {shift}", an[i], bn[i]);
            }
        }
        assert!(escaped > 0);
    }

    // Outside the contract: a shallower frame (larger |dc|), the bound column, the
    // scaled tier, another centre (the table's orbit is bound to its exact centre),
    // progressive refinement.
    let out = dir.join("refused.fds");
    let o = out.to_str().unwrap();
    for (extra, why) in [
        (&["--width", "1e-16"][..], "dc_max"),
        (&["--columns", "nu,bound"][..], "bound"),
        (&["--kernel", "scaled"][..], "scaled"),
        (&["--re", "-0.743643887037158704752191506114775"][..], "centre"),
        (&["--refine", "8"][..], "refine"),
    ] {
        let mut args = [&["render"][..], &view("1e-20")[..]].concat();
        for pair in extra.chunks(2) {
            let at = args.iter().position(|a| *a == pair[0]).map(|p| p + 1);
            match at {
                Some(p) => args[p] = pair[1],
                None => args.extend_from_slice(pair),
            }
        }
        let r = fd(&[&args[..], &["--store", st, "--bla", bla.as_str(), "-o", o]].concat());
        assert!(!r.status.success(), "{extra:?} was accepted");
        assert!(String::from_utf8_lossy(&r.stderr).contains(why), "{extra:?}: {}", String::from_utf8_lossy(&r.stderr));
    }
    // No table is built for the scaled tier.
    let r = fd(&[&build[..], &["--kernel", "scaled"]].concat());
    assert!(!r.status.success() && String::from_utf8_lossy(&r.stderr).contains("scaled"));
    // eps above 2^-41 would let a block cover an iterate the plain kernel judges for
    // periodicity (|delta| > 1e-12 |z|): refused; 2^-41 itself is accepted.
    let r = fd(&[&build[..], &["--eps", "1e-12"]].concat());
    assert!(!r.status.success() && String::from_utf8_lossy(&r.stderr).contains("eps"), "{}", String::from_utf8_lossy(&r.stderr));
    ok(&[&build[..], &["--eps", "4.547473508864641e-13"]].concat());

    // Samples settled by the closed-form main-cardioid test are neither BLA nor
    // fallback work: counted apart, and left out of fallback.sample_fraction.
    let wide = ["--re", "-0.2", "--im", "0", "--width", "1", "--size", "16x9", "--iter", "1000", "--columns", "nu,de"];
    let id = get(&ok(&[&["orbit", "bla", "--store", st][..], &wide[..]].concat()), "bla");
    let text = ok(&[&["render"][..], &wide[..], &["--store", st, "--bla", id.as_str(), "-o", o]].concat());
    let (closed, fallback): (u64, u64) = (get(&text, "closed_form.samples").parse().unwrap(), get(&text, "fallback.samples").parse().unwrap());
    assert!(closed > 0 && closed + fallback == 144, "{text}");
    assert_eq!(get(&text, "fallback.sample_fraction"), "1.000000", "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}
