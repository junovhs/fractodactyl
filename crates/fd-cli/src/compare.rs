//! `fd compare`: sample-by-sample agreement of two directories of frames (BENC-01). For
//! every `frame-NNNNN.fds` in the reference directory `A` (e.g. `fd control -o A`) it
//! reads the same file in `B` (`fd control --bla per-frame -o B`, `fd play -o B`) and
//! reports class agreement (exact: every sample's kind must match) and, over samples
//! both files class as escaped, how far `nu` moved, as the oracle measures it
//! (SAMPLES.md "Oracle tolerances"): an equivalent displacement in output pixels,
//! `|dnu| / |grad nu|` with `|grad nu| = 2 / (de ln 2)` per pixel (`de` from `A`).
//! A sample whose displacement exceeds `--px` (default 1e-3, the oracle's) is outside
//! the contract. A non-finite `nu` on an escaped sample (either side), or a non-finite or
//! negative `de` on an escaped reference sample, is its own failure (`non_finite`), never
//! folded into the maxima. Output: one fd-compare/1 JSON line per frame, then totals; exit 1
//! when any class differs, any sample is outside `--px` or non-finite, or a frame is missing
//! or differs in grid or view. Widths are compared as exact decimals at any depth (DEC-21):
//! they may differ by a relative 1e-15 (the player derives its width from the manifests);
//! `width_rel` reports it.
//!
//! When both files carry `de` (or `normal`), they are scored too on samples both class as
//! escaped (GATE-02): `de` relative error and normal angle error, failing above `--de-tol`
//! (default 0.2%) or `--normal-tol` (default 0.2 degrees), on samples whose reference `de`
//! exceeds 1e-3 px. Samples within 1e-3 px of the boundary, where both are ill-conditioned
//! (BENC-04), are reported under `near_boundary` but never fail.
use crate::args::Args;
use crate::bench::q;
use fd_samples::{Column, ColumnSet, Kind, Reader, Samples};
use std::fmt::Write as _;
use std::path::Path;

const USAGE: &str = "usage: fd compare A_DIR B_DIR [--px P] [--de-tol R] [--normal-tol DEG] [--frames A..B]";

/// Reference `de` (output px) at or below which de/normal differences are only reported.
const NEAR_PX: f32 = 1e-3;

/// Failure thresholds.
#[derive(Clone, Copy)]
struct Tol {
    /// `nu` displacement, output px.
    px: f64,
    /// `de` relative error.
    de: f64,
    /// Normal angle error, degrees.
    normal: f64,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["px", "de-tol", "normal-tol", "frames"])?;
    let [da, db] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let tol = Tol { px: a.num("px", 1e-3)?, de: a.num("de-tol", 2e-3)?, normal: a.num("normal-tol", 0.2)? };
    let px = tol.px;
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
        let j = match frame(&Path::new(da).join(name), &Path::new(db).join(name), tol) {
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
    let ok = t.errors == 0 && t.c.ok();
    let mut j = format!(
        "{{\"schema\":\"fd-compare/1\",\"record\":\"totals\",\"a\":{},\"b\":{},\"px\":{px},\"de_tol\":{},\"normal_tol\":{}",
        q(da),
        q(db),
        tol.de,
        tol.normal
    );
    let _ = write!(
        j,
        ",\"frames\":{},\"errors\":{},\"frames_class_identical\":{},\"frames_bytes_identical\":{},\"frames_width_off\":{},\"samples\":{}",
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
    /// Escaped samples with a non-finite `nu` (A or B) or a non-finite/negative `de` (A).
    non_finite: u64,
    /// Escaped samples whose integer escape count (`floor nu` + 1) differs.
    count_changed: u64,
    bytes_identical: bool,
    /// Relative difference between the two widths (at most `WIDTH_REL`).
    width_rel: f64,
    /// `de` relative error and normal angle error (degrees), when both files have them.
    de: Field,
    normal: Field,
}

/// Agreement of one secondary column on samples both escaped.
#[derive(Default)]
struct Field {
    /// Frames where both files carry the column.
    frames: u64,
    /// Samples scored (reference `de` above `NEAR_PX`).
    compared: u64,
    max: f64,
    over: u64,
    /// Samples within `NEAR_PX` of the boundary: reported, never failing.
    near: u64,
    near_max: f64,
}

impl Field {
    fn add(&mut self, err: f64, near: bool, tol: f64) {
        if near {
            self.near += 1;
            self.near_max = self.near_max.max(err);
        } else {
            self.compared += 1;
            self.max = self.max.max(err);
            self.over += u64::from(err.is_nan() || err > tol);
        }
    }

    fn sum(&mut self, o: &Field) {
        self.frames += o.frames;
        self.compared += o.compared;
        self.max = self.max.max(o.max);
        self.over += o.over;
        self.near += o.near;
        self.near_max = self.near_max.max(o.near_max);
    }

    fn json(&self, j: &mut String, name: &str, unit: &str) {
        let _ = write!(
            j,
            ",\"{name}\":{{\"frames_scored\":{},\"compared\":{},\"{unit}_max\":{},\"over_tol\":{},\"near_boundary\":{{\"samples\":{},\"{unit}_max\":{}}}}}",
            self.frames, self.compared, self.max, self.over, self.near, self.near_max
        );
    }
}

/// Smallest angle between two stored normal angles (65536 per turn), in degrees.
fn angle_deg(a: u16, b: u16) -> f64 {
    f64::from(a.wrapping_sub(b).min(b.wrapping_sub(a))) * (360.0 / 65536.0)
}

fn kind(s: &Samples, k: usize) -> usize {
    match s.class[k].kind() {
        Some(Kind::Escaped) => 0,
        Some(Kind::Interior) => 1,
        _ => 2,
    }
}

/// Largest relative width difference accepted as the same view.
const WIDTH_REL: f64 = 1e-15;

/// A decimal string as sign, mantissa in [1, 10) (17 significant digits) and a base-10
/// exponent of any size; `None` mantissa for zero. Never goes through an f64 of the value.
fn decimal(s: &str) -> Result<(bool, Option<(f64, i64)>), String> {
    let bad = || format!("bad width {s:?}");
    let s = s.trim();
    let (neg, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let (m, e) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], s[i + 1..].parse::<i64>().map_err(|_| bad())?),
        None => (s, 0),
    };
    let (int, frac) = m.split_once('.').unwrap_or((m, ""));
    let digits: String = [int, frac].concat();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let Some(lead) = digits.bytes().position(|b| b != b'0') else { return Ok((neg, None)) };
    let sig = &digits[lead..digits.len().min(lead + 17)];
    let mant: f64 = format!("{}.{}", &sig[..1], &sig[1..]).parse().map_err(|_| bad())?;
    let exp = e.checked_sub(frac.len() as i64).and_then(|x| x.checked_add((digits.len() - lead - 1) as i64)).ok_or_else(bad)?;
    Ok((neg, Some((mant, exp))))
}

/// Relative difference of two decimal widths; infinite when sign or magnitude differ.
fn width_rel(a: &str, b: &str) -> Result<f64, String> {
    let ((sa, a), (sb, b)) = (decimal(a)?, decimal(b)?);
    Ok(match (a, b) {
        (None, None) => 0.0,
        (Some((ma, ea)), Some((mb, eb))) if sa == sb && (ea - eb).abs() <= 1 => {
            let mb = mb * 10f64.powi((eb - ea) as i32);
            (ma - mb).abs() / ma.max(mb)
        }
        _ => f64::INFINITY,
    })
}

fn frame(pa: &Path, pb: &Path, tol: Tol) -> Result<Cmp, String> {
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
    // which may sit an f64 ulp or two off the path's; that moves no sample by a resolvable
    // amount (relative 1e-16 of the frame). Widths are compared as exact decimals, so
    // 1e-1000 and 1e-2000 differ even though both underflow an f64 (DEC-21).
    let rel = width_rel(&ha.view.width, &hb.view.width)?;
    if rel > WIDTH_REL {
        return Err(format!("widths differ: {} vs {}", ha.view.width, hb.view.width));
    }
    // A must carry nu and de (de converts nu to px); de and normal of B, and normal of
    // A, are read when present and scored when both files have them.
    let (ca, cb) = (ha.columns, hb.columns);
    let base = ColumnSet::of(&[Column::Class, Column::Nu, Column::De]);
    let want_a = if ca.has(Column::Normal) { base.with(Column::Normal) } else { base };
    let mut want_b = ColumnSet::of(&[Column::Class, Column::Nu]);
    for col in [Column::De, Column::Normal] {
        if cb.has(col) {
            want_b = want_b.with(col);
        }
    }
    let (sa, sb) = (ra.read(want_a).map_err(|e| e.to_string())?, rb.read(want_b).map_err(|e| e.to_string())?);
    let (na, nb, de) = (sa.nu.as_ref().ok_or("A has no nu")?, sb.nu.as_ref().ok_or("B has no nu")?, sa.de.as_ref().ok_or("A has no de")?);
    let de_b = sb.de.as_ref();
    let normals = sa.normal.as_ref().zip(sb.normal.as_ref());
    let mut c = Cmp { samples: sa.class.len() as u64, width_rel: rel, ..Cmp::default() };
    c.de.frames = u64::from(de_b.is_some());
    c.normal.frames = u64::from(normals.is_some());
    for k in 0..sa.class.len() {
        let (x, y) = (kind(&sa, k), kind(&sb, k));
        c.kinds[x][y] += 1;
        let a_bad = x == 0 && !(na[k].is_finite() && de[k].is_finite() && de[k] >= 0.0);
        let b_bad = y == 0 && !(nb[k].is_finite() && de_b.is_none_or(|d| d[k].is_finite() && d[k] >= 0.0));
        if a_bad || b_bad {
            c.non_finite += 1;
        }
        if x != y {
            c.class_mismatches += 1;
        } else if x == 0 {
            c.both_escaped += 1;
            if a_bad || b_bad {
                continue;
            }
            let d = (na[k] - nb[k]).abs();
            if d == 0.0 {
                c.nu_equal += 1;
            }
            c.count_changed += u64::from(na[k].floor() != nb[k].floor());
            c.nu_abs_max = c.nu_abs_max.max(d);
            let disp = d * f64::from(de[k]) * std::f64::consts::LN_2 / 2.0;
            c.nu_px_max = c.nu_px_max.max(disp);
            c.nu_over += u64::from(disp > tol.px);
            let near = de[k] <= NEAR_PX;
            if let Some(db) = de_b {
                let rel = (f64::from(db[k]) - f64::from(de[k])).abs() / f64::from(de[k]);
                c.de.add(if de[k] == 0.0 && db[k] == 0.0 { 0.0 } else { rel }, near, tol.de);
            }
            if let Some((ma, mb)) = normals {
                c.normal.add(angle_deg(ma[k], mb[k]), near, tol.normal);
            }
        }
    }
    c.bytes_identical = std::fs::read(pa).map_err(|e| e.to_string())? == std::fs::read(pb).map_err(|e| e.to_string())?;
    Ok(c)
}

impl Cmp {
    fn ok(&self) -> bool {
        self.class_mismatches == 0 && self.nu_over == 0 && self.non_finite == 0 && self.de.over == 0 && self.normal.over == 0
    }

    fn json(&self, f: usize, name: &str) -> String {
        let ok = self.ok();
        let mut j = format!("{{\"schema\":\"fd-compare/1\",\"record\":\"frame\",\"frame\":{f},\"file\":{},\"samples\":{}", q(name), self.samples);
        let _ = write!(j, ",\"bytes_identical\":{},\"width_rel\":{}", self.bytes_identical, self.width_rel);
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
        let _ = write!(j, ",\"non_finite\":{}", self.non_finite);
        let _ = write!(
            j,
            ",\"nu\":{{\"both_escaped\":{},\"equal\":{},\"abs_max\":{},\"px_max\":{},\"over_px\":{},\"escape_count_changed\":{}}}",
            self.both_escaped, self.nu_equal, self.nu_abs_max, self.nu_px_max, self.nu_over, self.count_changed
        );
        self.de.json(j, "de", "rel");
        self.normal.json(j, "normal", "deg");
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
        s.non_finite += c.non_finite;
        s.de.sum(&c.de);
        s.normal.sum(&c.normal);
        s.count_changed += c.count_changed;
        self.class_identical += usize::from(c.class_mismatches == 0);
        self.bytes_identical += usize::from(c.bytes_identical);
        self.width_off += usize::from(c.width_rel != 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{angle_deg, width_rel};

    #[test]
    fn normal_angles_wrap() {
        assert_eq!(angle_deg(10, 10), 0.0);
        assert!((angle_deg(0, 65535) - 360.0 / 65536.0).abs() < 1e-12);
        assert!((angle_deg(65535, 0) - 360.0 / 65536.0).abs() < 1e-12);
        assert!((angle_deg(0, 32768) - 180.0).abs() < 1e-12);
    }

    #[test]
    fn widths_compare_as_exact_decimals_at_any_depth() {
        assert!(width_rel("1e-1000", "1e-2000").unwrap().is_infinite());
        assert!(width_rel("1.0000000000000001e-1000", "1e-1000").unwrap() <= 1e-15);
        assert_eq!(width_rel("0.00025", "2.5e-4").unwrap(), 0.0);
        assert_eq!(width_rel("9.9999999999999999e-5", "1e-4").unwrap(), 0.0);
        assert!(width_rel("1e-40", "1.000000000000002e-40").unwrap() > 1e-15);
        assert!(width_rel("1e-40", "-1e-40").unwrap().is_infinite());
        assert_eq!(width_rel("0", "0.000").unwrap(), 0.0);
        assert!(width_rel("1e-40", "1e-41").unwrap() > 0.5);
        assert!(width_rel("abc", "1").is_err() && width_rel("1e", "1").is_err() && width_rel(".", "1").is_err());
    }
}
