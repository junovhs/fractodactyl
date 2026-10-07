//! `fd compare`: sample-by-sample agreement of two directories of frames (BENC-01). For
//! every `frame-NNNNN.fds` in the reference directory `A` (e.g. `fd control -o A`) it
//! reads the same file in `B` (`fd control --bla per-frame -o B`, `fd play -o B`) and
//! reports class agreement (exact: every sample's kind must match) and, over samples
//! both files class as escaped, how far `nu` moved, as the oracle measures it
//! (SAMPLES.md "Oracle tolerances"): an equivalent displacement in output pixels,
//! `|dnu| / |grad nu|` with `|grad nu| = 2 / (de ln 2)` per pixel (`de` from `A`).
//! A sample whose displacement exceeds `--px` (default 1e-3, the oracle's) is outside
//! the contract. Output: one fd-compare/1 JSON line per frame, then totals; exit 1 when
//! any class differs, any sample is outside `--px`, or a frame is missing or differs in
//! grid or view (widths may differ by at most 2 f64 ulps: the player derives its width
//! from the manifests; `width_ulps` reports it).
use crate::args::Args;
use crate::bench::q;
use fd_samples::{Column, ColumnSet, Kind, Reader, Samples};
use std::fmt::Write as _;
use std::path::Path;

const USAGE: &str = "usage: fd compare A_DIR B_DIR [--px P] [--frames A..B]";

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["px", "frames"])?;
    let [da, db] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let px: f64 = a.num("px", 1e-3)?;
    let mut names: Vec<String> = std::fs::read_dir(da)
        .map_err(|e| format!("{da}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|n| n.starts_with("frame-") && n.ends_with(".fds"))
        .collect();
    names.sort();
    if let Some(r) = a.str("frames") {
        let bad = || format!("--frames: expected A..B, got {r:?}");
        let (x, y) = r.split_once("..").ok_or_else(bad)?;
        let x: usize = x.parse().map_err(|_| bad())?;
        let y: usize = if y.is_empty() { usize::MAX } else { y.parse().map_err(|_| bad())? };
        names.retain(|n| n[6..n.len() - 4].parse::<usize>().is_ok_and(|f| f >= x && f < y));
    }
    if names.is_empty() {
        return Err(format!("{da}: no frame-NNNNN.fds files"));
    }
    let mut t = Sum::default();
    for name in &names {
        let f: usize = name[6..name.len() - 4].parse().map_err(|_| format!("{name}: bad frame number"))?;
        let j = match frame(&Path::new(da).join(name), &Path::new(db).join(name), px) {
            Ok(c) => {
                let j = c.json(f, name);
                t.add(&c);
                j
            }
            Err(e) => {
                t.errors += 1;
                format!("{{\"schema\":\"fd-compare/1\",\"record\":\"frame\",\"frame\":{f},\"error\":{},\"ok\":false}}", q(&e))
            }
        };
        println!("{j}");
    }
    let ok = t.errors == 0 && t.c.class_mismatches == 0 && t.c.nu_over == 0;
    let mut j = format!("{{\"schema\":\"fd-compare/1\",\"record\":\"totals\",\"a\":{},\"b\":{},\"px\":{px}", q(da), q(db));
    let _ = write!(
        j,
        ",\"frames\":{},\"errors\":{},\"frames_class_identical\":{},\"frames_bytes_identical\":{},\"frames_width_ulp_off\":{},\"samples\":{}",
        names.len(),
        t.errors,
        t.class_identical,
        t.bytes_identical,
        t.width_off,
        t.c.samples
    );
    t.c.body(&mut j);
    let _ = write!(j, ",\"ok\":{ok}}}");
    println!("{j}");
    if !ok {
        std::process::exit(1);
    }
    Ok(())
}

/// Agreement of one pair of frames (or sums over many).
#[derive(Default)]
struct Cmp {
    samples: u64,
    /// `[kind in A][kind in B]`: escaped, interior, unresolved.
    kinds: [[u64; 3]; 3],
    class_mismatches: u64,
    both_escaped: u64,
    nu_equal: u64,
    nu_abs_max: f64,
    nu_px_max: f64,
    nu_over: u64,
    /// Escaped samples whose integer escape count (`floor nu` + 1) differs.
    count_changed: u64,
    bytes_identical: bool,
    /// f64 ulps between the two widths (0 or at most 2).
    width_ulps: u64,
}

fn kind(s: &Samples, k: usize) -> usize {
    match s.class[k].kind() {
        Some(Kind::Escaped) => 0,
        Some(Kind::Interior) => 1,
        _ => 2,
    }
}

fn frame(pa: &Path, pb: &Path, px: f64) -> Result<Cmp, String> {
    let open = |p: &Path| Reader::open(p).map_err(|e| format!("{}: {e}", p.display()));
    let (mut ra, mut rb) = (open(pa)?, open(pb)?);
    let (ha, hb) = (&ra.header, &rb.header);
    if (ha.nx, ha.ny, ha.ss, ha.max_iter) != (hb.nx, hb.ny, hb.ss, hb.max_iter) {
        return Err("grids differ".into());
    }
    if ha.view.rotation != hb.view.rotation || ha.view.center_re != hb.view.center_re || ha.view.center_im != hb.view.center_im {
        return Err("views differ".into());
    }
    // The player writes the width it derives from the manifests (`width x 2^(2 - L)`),
    // which may sit an ulp or two off the path's; that moves no sample by a resolvable
    // amount (relative 1e-16 of the frame). More than 2 ulps is a different view.
    let w = |s: &str| s.parse::<f64>().map_err(|_| format!("bad width {s:?}"));
    let (wa, wb) = (w(&ha.view.width)?, w(&hb.view.width)?);
    let ulps = (wa.to_bits() as i64 - wb.to_bits() as i64).unsigned_abs();
    if wa.is_sign_negative() != wb.is_sign_negative() || ulps > 2 {
        return Err(format!("widths differ: {} vs {}", ha.view.width, hb.view.width));
    }
    let want = ColumnSet::of(&[Column::Class, Column::Nu, Column::De]);
    let (sa, sb) = (ra.read(want).map_err(|e| e.to_string())?, rb.read(want).map_err(|e| e.to_string())?);
    let (na, nb, de) = (sa.nu.as_ref().ok_or("A has no nu")?, sb.nu.as_ref().ok_or("B has no nu")?, sa.de.as_ref().ok_or("A has no de")?);
    let mut c = Cmp { samples: sa.class.len() as u64, width_ulps: ulps, ..Cmp::default() };
    for k in 0..sa.class.len() {
        let (x, y) = (kind(&sa, k), kind(&sb, k));
        c.kinds[x][y] += 1;
        if x != y {
            c.class_mismatches += 1;
        } else if x == 0 {
            c.both_escaped += 1;
            let d = (na[k] - nb[k]).abs();
            if d == 0.0 {
                c.nu_equal += 1;
            }
            c.count_changed += u64::from(na[k].floor() != nb[k].floor());
            c.nu_abs_max = c.nu_abs_max.max(d);
            let disp = d * f64::from(de[k]) * std::f64::consts::LN_2 / 2.0;
            c.nu_px_max = c.nu_px_max.max(disp);
            c.nu_over += u64::from(disp > px);
        }
    }
    c.bytes_identical = std::fs::read(pa).map_err(|e| e.to_string())? == std::fs::read(pb).map_err(|e| e.to_string())?;
    Ok(c)
}

impl Cmp {
    fn json(&self, f: usize, name: &str) -> String {
        let ok = self.class_mismatches == 0 && self.nu_over == 0;
        let mut j = format!("{{\"schema\":\"fd-compare/1\",\"record\":\"frame\",\"frame\":{f},\"file\":{},\"samples\":{}", q(name), self.samples);
        let _ = write!(j, ",\"bytes_identical\":{},\"width_ulps\":{}", self.bytes_identical, self.width_ulps);
        self.body(&mut j);
        let _ = write!(j, ",\"ok\":{ok}}}");
        j
    }

    fn body(&self, j: &mut String) {
        let k = &self.kinds;
        let _ = write!(
            j,
            ",\"class_mismatches\":{},\"kinds\":{{\"escaped_to_interior\":{},\"escaped_to_unresolved\":{},\"interior_to_escaped\":{},\"interior_to_unresolved\":{},\"unresolved_to_escaped\":{},\"unresolved_to_interior\":{}}}",
            self.class_mismatches, k[0][1], k[0][2], k[1][0], k[1][2], k[2][0], k[2][1]
        );
        let _ = write!(
            j,
            ",\"nu\":{{\"both_escaped\":{},\"equal\":{},\"abs_max\":{},\"px_max\":{},\"over_px\":{},\"escape_count_changed\":{}}}",
            self.both_escaped, self.nu_equal, self.nu_abs_max, self.nu_px_max, self.nu_over, self.count_changed
        );
    }
}

#[derive(Default)]
struct Sum {
    c: Cmp,
    errors: usize,
    class_identical: usize,
    bytes_identical: usize,
    width_off: usize,
}

impl Sum {
    fn add(&mut self, c: &Cmp) {
        let s = &mut self.c;
        s.samples += c.samples;
        for (x, y) in s.kinds.iter_mut().flatten().zip(c.kinds.iter().flatten()) {
            *x += y;
        }
        s.class_mismatches += c.class_mismatches;
        s.both_escaped += c.both_escaped;
        s.nu_equal += c.nu_equal;
        s.nu_abs_max = s.nu_abs_max.max(c.nu_abs_max);
        s.nu_px_max = s.nu_px_max.max(c.nu_px_max);
        s.nu_over += c.nu_over;
        s.count_changed += c.count_changed;
        self.class_identical += usize::from(c.class_mismatches == 0);
        self.bytes_identical += usize::from(c.bytes_identical);
        self.width_off += usize::from(c.width_ulps != 0);
    }
}
