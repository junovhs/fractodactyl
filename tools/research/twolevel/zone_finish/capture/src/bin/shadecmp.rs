//! Shading proof for the DE contract: render the binary64 zone frame and candidate frames
//! with every built-in look and count changed pixels and colour levels.
//! usage: shadecmp WIDTH DUMP LOOKSDIR
//!   DUMP: per captured pixel (capture order) 3 arms x 4 f64 {n, nu, de, angle}:
//!         binary64, float-float, hybrid-B (from zfin.c with DUMPOUT=).
use fd_samples::{Column, ColumnSet, Header, Kind, Samples, View};
use fd_shade::Pass;
use std::io::Read;

fn quant(angle: f64) -> u16 {
    (angle * (65536.0 / std::f64::consts::TAU)).round() as i64 as u16
}

fn diff(a: &fd_shade::Rgb8, b: &fd_shade::Rgb8) -> (usize, u8) {
    let mut n = 0;
    let mut m = 0u8;
    for (pa, pb) in a.data.chunks(3).zip(b.data.chunks(3)) {
        let d = pa.iter().zip(pb).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
        if d > 0 {
            n += 1;
            m = m.max(d);
        }
    }
    (n, m)
}

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let width = a[1].as_str();
    let zone = fd_kernel::Zone::parse(include_str!("../../../../../../../bench/zones/v0-core.zone"))?;
    let p = fd_kernel::Params {
        nx: 1920, ny: 1080, ss: 1, max_iter: 20_000, escape_radius: 1e10,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        threads: 1, tier: None,
    };
    let view = View {
        center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
        center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
        width: width.into(), rotation: 0.0,
    };
    let (h, refs, states, _c): (Header, Samples, _, _) = zone.probe_finish_inputs(&view, &p)?;
    let idx: Vec<usize> = states.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i).collect();
    let mut buf = Vec::new();
    std::fs::File::open(&a[2]).map_err(|e| e.to_string())?.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let vals: Vec<f64> = buf.chunks(8).map(|c| f64::from_le_bytes(c.try_into().unwrap())).collect();
    if vals.len() != idx.len() * 16 { return Err(format!("dump has {} values, want {}", vals.len(), idx.len() * 16)); }
    let arm = |k: usize, j: usize, f: usize| vals[k * 16 + j * 4 + f];

    // sanity: the binary64 arm must reproduce the CPU zone columns
    let (rnu, rde, rnm) = (refs.nu.as_ref().unwrap(), refs.de.as_ref().unwrap(), refs.normal.as_ref().unwrap());
    let (mut dnu, mut dde, mut dnm) = (0f64, 0f64, 0u16);
    for (k, &i) in idx.iter().enumerate() {
        if refs.class[i].kind() != Some(Kind::Escaped) || arm(k, 0, 0) < 0.0 { continue; }
        dnu = dnu.max((arm(k, 0, 1) - rnu[i]).abs());
        dde = dde.max(((arm(k, 0, 2) as f32 - rde[i]).abs() / rde[i]) as f64);
        let q = quant(arm(k, 0, 3));
        dnm = dnm.max(q.wrapping_sub(rnm[i]).min(rnm[i].wrapping_sub(q)));
    }
    println!("sanity width={width} binary64-vs-zone max |dnu| {dnu:.2e} max rel dDE {dde:.2e} max dnormal units {dnm}");

    // Image samples built from arms. Columns of captured escaped pixels are
    // base + amp * (err - base) for the columns in `mask` (1 nu, 2 de, 4 normal) and only where
    // the truth DE < gate; otherwise base. Arms: 0 binary64, 1 float-float, 2 hybrid-B, 3 double-double.
    // Normals are carried as deltas from the binary64 arm onto the reference's u16 encoding.
    let build = |base: usize, err: usize, amp: f64, gate: f64, mask: u32| -> Samples {
        let mut s = refs.clone();
        let (nu, de, nm) = (s.nu.as_mut().unwrap(), s.de.as_mut().unwrap(), s.normal.as_mut().unwrap());
        for (k, &i) in idx.iter().enumerate() {
            if refs.class[i].kind() != Some(Kind::Escaped) { continue; }
            if arm(k, 0, 0) < 0.0 || arm(k, base, 0) < 0.0 || arm(k, err, 0) < 0.0 { continue; }
            let v = |j: usize, f: usize| arm(k, j, f);
            let use_err = v(3, 2) < gate;
            let pick = |f: usize, bit: u32| if use_err && mask & bit != 0 { v(base, f) + amp * (v(err, f) - v(base, f)) } else { v(base, f) };
            nu[i] = rnu[i] + (pick(1, 1) - v(0, 1));
            de[i] = (rde[i] as f64 * pick(2, 2) / v(0, 2)) as f32;
            let mut da = pick(3, 4) - v(0, 3);
            da -= (da / std::f64::consts::TAU).round() * std::f64::consts::TAU;
            nm[i] = rnm[i].wrapping_add(quant(da));
        }
        s
    };
    // truth DE per output pixel (for bucketed reporting)
    let mut tde = vec![f64::INFINITY; refs.class.len()];
    for (k, &i) in idx.iter().enumerate() { if arm(k, 3, 0) >= 0.0 { tde[i] = arm(k, 3, 2); } }
    let mut looks: Vec<(String, Box<dyn Pass>)> = Vec::new();
    for n in fd_shade::NAMES { looks.push((n.to_string(), fd_shade::by_name(n).unwrap())); }
    for (n, _) in fd_shade::BUILTIN { looks.push((format!("look:{n}"), Box::new(fd_shade::Studio(fd_shade::Look::builtin(n).unwrap())))); }
    if a.len() > 3 {
        for e in std::fs::read_dir(&a[3]).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            if p.extension().map(|x| x == "look").unwrap_or(false) {
                let t = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
                looks.push((format!("file:{}", p.file_name().unwrap().to_string_lossy()), Box::new(fd_shade::Studio(fd_shade::Look::parse(&t)?))));
            }
        }
    }
    let truth = build(3, 3, 0.0, 0.0, 0);
    let imgs: Vec<fd_shade::Rgb8> = looks.iter().map(|(_, l)| l.shade(&h, &truth)).collect();
    let buckets = [0.0, 1e-3, 1e-2, 1e-1, 1.0, f64::INFINITY];
    let report = |label: &str, s: &Samples| {
        let mut cnt = [0usize; 5]; let mut mx = [0u8; 5];
        for ((_, l), base) in looks.iter().zip(&imgs) {
            let img = l.shade(&h, s);
            for (p, (pa, pb)) in base.data.chunks(3).zip(img.data.chunks(3)).enumerate() {
                let d = pa.iter().zip(pb).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
                if d == 0 { continue; }
                let b = (0..5).find(|&b| tde[p] < buckets[b + 1]).unwrap_or(4);
                cnt[b] += 1; mx[b] = mx[b].max(d);
            }
        }
        println!("  {label:52} changed px (summed over looks) / max levels by truth DE: <1e-3 {}/{} | 1e-3..1e-2 {}/{} | 1e-2..0.1 {}/{} | 0.1..1 {}/{} | >=1 {}/{}",
                 cnt[0], mx[0], cnt[1], mx[1], cnt[2], mx[2], cnt[3], mx[3], cnt[4], mx[4]);
    };
    println!("width={width} looks={} (images compared against the double-double truth image)", looks.len());
    report("binary64 (production) vs truth, all columns", &build(3, 0, 1.0, f64::INFINITY, 7));
    report("float-float vs truth, all columns", &build(3, 1, 1.0, f64::INFINITY, 7));
    report("hybrid-B vs truth, all columns", &build(3, 2, 1.0, f64::INFINITY, 7));
    report("float-float vs truth, nu only", &build(3, 1, 1.0, f64::INFINITY, 1));
    report("float-float vs truth, DE+normal only", &build(3, 1, 1.0, f64::INFINITY, 6));
    for amp in [1.0, 3.0, 10.0, 30.0] {
        for gate in [1e-3, 1e-2, 0.1, 1.0, 10.0, f64::INFINITY] {
            report(&format!("ff DE+normal errors x{amp} where DE<{gate:e}"), &build(3, 1, amp, gate, 6));
        }
    }
    Ok(())
}
