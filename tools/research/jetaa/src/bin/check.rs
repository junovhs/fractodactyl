//! Jet accuracy: predict the four ss 2 subsamples from the centre jet and compare with the
//! true binary64 subsamples. usage: check DUMP...
//! Errors by centre DE band (output px): |dnu| (nu units), |rel dDE|, |d angle| (rad), at jet
//! orders 0 (copy the centre), 1, 2 and 3. Also how often the true subsample is unresolved.
use jetaa::*;

fn pct(v: &mut [f64], q: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let k = ((v.len() as f64 - 1.0) * q).round() as usize;
    v.select_nth_unstable_by(k, |a, b| a.total_cmp(b));
    v[k]
}

fn main() -> Result<(), String> {
    let bands = [0.0, 0.03, 0.1, 0.3, 1.0, 3.0, f64::INFINITY];
    for path in std::env::args().skip(1) {
        let d = Dump::load(&path)?;
        println!("== {path} {}x{}", d.w, d.h);
        // [band][order] -> (dnu, dde, dang)
        let mut e: Vec<Vec<[Vec<f64>; 3]>> = (0..6).map(|_| (0..4).map(|_| [vec![], vec![], vec![]]).collect()).collect();
        let mut lost = [0usize; 6];
        let mut tot = [0usize; 6];
        for p in 0..d.w * d.h {
            let r = d.rec(p);
            let Some(j) = Jet::from_raw(&r[..CENTRE]) else { continue };
            let c0 = columns(&r[..CENTRE]).unwrap();
            let b = (0..6).find(|&b| (c0.1 as f64) < bands[b + 1]).unwrap();
            for k in 0..4 {
                let (dx, dy) = ((k % 2) as f64 * 0.5 - 0.25, (k / 2) as f64 * 0.5 - 0.25);
                tot[b] += 1;
                let s = &r[CENTRE + 5 * k..CENTRE + 5 * k + 5];
                let Some(t) = columns(s) else {
                    lost[b] += 1;
                    continue;
                };
                for o in 0..4u32 {
                    let (nu, de, x, y) = if o == 0 { c0 } else { j.at(dx, dy, o) };
                    let da = (y.atan2(x) - t.3.atan2(t.2) + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
                    let v = &mut e[b][o as usize];
                    let fix = |x: f64| if x.is_nan() { f64::INFINITY } else { x };
                    v[0].push(fix((nu - t.0).abs()));
                    v[1].push(fix(((de - t.1) / t.1).abs() as f64));
                    v[2].push(da.abs());
                }
            }
        }
        for b in 0..6 {
            if tot[b] == 0 {
                continue;
            }
            println!(
                " DE {:>5}..{:<5} subsamples {:>8} true-unresolved {:>6}",
                bands[b], bands[b + 1], tot[b], lost[b]
            );
            for o in 0..4 {
                let v = &mut e[b][o];
                let line: Vec<String> = (0..3)
                    .map(|f| {
                        format!(
                            "p50 {:.1e} p99 {:.1e} p99.9 {:.1e} max {:.1e}",
                            pct(&mut v[f], 0.5),
                            pct(&mut v[f], 0.99),
                            pct(&mut v[f], 0.999),
                            pct(&mut v[f], 1.0)
                        )
                    })
                    .collect();
                println!("   order {o}: dnu [{}] | rel dDE [{}] | dangle [{}]", line[0], line[1], line[2]);
            }
        }
    }
    Ok(())
}
