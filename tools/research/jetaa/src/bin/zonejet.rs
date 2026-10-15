//! Jet-AA through the zone fast path, end to end on fd's own output: render a v0 frame with
//! `fd_kernel::render_zone` at 1920x1080 ss 2 (the exact film render), keep each pixel's
//! top-left sample as its one real sample, rebuild the other three from the stored columns
//! of the real samples only (corner scheme, neighbour stencil, consistency check `m`) and
//! compare the shaded frames.
//! usage: zonejet WIDTH [M...]
use fd_samples::{Class, Column, ColumnSet, Evidence, Kind, Samples, View};
use fd_shade::{Appearance, Pass};
use jetaa::*;

const FILM: Appearance = Appearance { aa: true, unresolved_interior: true, ..Appearance::STILL };

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    let width = a[1].as_str();
    let ms: Vec<f64> = if a.len() > 2 { a[2..].iter().map(|x| x.parse().unwrap()).collect() } else { vec![1.0, 3.0, 10.0] };
    let zone = fd_kernel::Zone::parse(include_str!("../../../../../bench/zones/v0-core.zone"))?;
    let (w, h) = (1920usize, 1080usize);
    let p = fd_kernel::Params {
        nx: 2 * w as u32, ny: 2 * h as u32, ss: 2, max_iter: 100_000, escape_radius: 1e10,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        threads: std::thread::available_parallelism().map_or(1, |n| n.get()), tier: None,
    };
    let view = View {
        center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
        center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
        width: width.into(), rotation: 0.0,
    };
    if !fd_kernel::zone_covers(&view, &p, &zone)? {
        return Err(format!("zone does not cover width {width}"));
    }
    let t = std::time::Instant::now();
    let (hd, ex, _st, zs) = fd_kernel::render_zone(&view, &p, &zone)?;
    let secs = t.elapsed().as_secs_f64();
    let (nu, de, nm) = (ex.nu.as_ref().unwrap(), ex.de.as_ref().unwrap(), ex.normal.as_ref().unwrap());
    let idx = |px: usize, py: usize, k: usize| (2 * py + k / 2) * 2 * w + 2 * px + k % 2;
    let esc = |i: usize| ex.class[i].kind() == Some(Kind::Escaped);
    // real-sample raw records, rebuilt from columns
    let reals: Vec<Option<[f64; 9]>> = (0..w * h)
        .map(|q| {
            let i = idx(q % w, q / w, 0);
            esc(i).then(|| raw_from_columns(nu[i], de[i], nm[i]))
        })
        .collect();
    let jet = |q: usize| -> Option<Jet> {
        let (x, y) = (q % w, q / w);
        if x == 0 || y == 0 || x + 1 >= w || y + 1 >= h {
            return None;
        }
        let r = |q: usize| reals[q];
        let (c, rt, lf, up, dn) = (r(q)?, r(q + 1)?, r(q - 1)?, r(q - w)?, r(q + w)?);
        jet_from_neighbours(&c, [&rt, &lf, &up, &dn])
    };
    let looks = {
        let mut v: Vec<(String, Box<dyn Pass>)> = Vec::new();
        for n in ["umber", "palette", "relief"] {
            v.push((n.into(), fd_shade::by_name(n).unwrap()));
        }
        for (n, _) in fd_shade::BUILTIN {
            v.push((n.into(), Box::new(fd_shade::Studio(fd_shade::Look::builtin(n).unwrap()))));
        }
        let mi = fd_shade::Look::parse(include_str!("../../../../../looks/master-ice.look")).unwrap();
        v.push(("master-ice".into(), Box::new(fd_shade::Studio(mi))));
        v
    };
    let ex_img: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&hd, &ex, &FILM)).collect();
    let interior = (0..w * h).filter(|&q| matches!(ex.class[idx(q % w, q / w, 0)].kind(), Some(Kind::Interior))).count();
    let mut low = [0usize; 3];
    for q in 0..w * h {
        let i = idx(q % w, q / w, 0);
        if esc(i) {
            for (k, t) in [0.3f32, 1.0, 3.0].iter().enumerate() {
                low[k] += (de[i] < *t) as usize;
            }
        }
    }
    println!(
        "zone v0 width {width}: 1920x1080 ss2 in {secs:.1}s (returns {} jumps {} plain {}), real-sample interior {:.2}%, de<0.3 {:.2}% <1 {:.2}% <3 {:.2}%",
        zs.returns, zs.jumps, zs.plain, 100.0 * interior as f64 / (w * h) as f64,
        100.0 * low[0] as f64 / (w * h) as f64, 100.0 * low[1] as f64 / (w * h) as f64, 100.0 * low[2] as f64 / (w * h) as f64
    );
    // ss1: the real samples copied to all four
    for m in std::iter::once(-1.0).chain(ms.iter().copied()) {
        let mut s = ex.clone();
        let mut extra = 0usize;
        for q in 0..w * h {
            let (px, py) = (q % w, q / w);
            let i0 = idx(px, py, 0);
            let border = px == 0 || py == 0 || px + 2 >= w || py + 2 >= h;
            let mut pred: [Option<Option<(f64, f32, f64, f64)>>; 4] = [None; 4];
            if m < 0.0 {
                // ss1 baseline: copy the real sample
                for k in 1..4 {
                    pred[k] = Some(if esc(i0) {
                        let (x, y) = Samples::unit(nm[i0]);
                        Some((nu[i0], de[i0], x as f64, y as f64))
                    } else {
                        None
                    });
                }
            } else if !border {
                if !esc(i0) {
                    let all = [q + 1, q + w, q + w + 1, q - 1, q - w].iter().all(|&r| !esc(idx(r % w, r / w, 0)));
                    if all {
                        for k in 1..4 {
                            pred[k] = Some(None);
                        }
                    }
                } else if let Some(j) = jet(q) {
                    let nb = [q + 1, q + w, q + w + 1];
                    for k in 1..4 {
                        let (ox, oy) = ((k % 2) as f64 * 0.5, (k / 2) as f64 * 0.5);
                        let Some(jn) = jet(nb[k - 1]) else { continue };
                        let a = j.at(ox, oy, 3);
                        let b = jn.at(ox - (k % 2) as f64, oy - (k / 2) as f64, 3);
                        let da = (a.3.atan2(a.2) - b.3.atan2(b.2) + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
                        if (a.0 - b.0).abs() < m * 1e-3 && ((a.1 / b.1) as f64).ln().abs() < m * 1e-2 && da.abs() < m * 3e-3 {
                            let (na, nb_) = (a.2.hypot(a.3), b.2.hypot(b.3));
                            pred[k] = Some(Some((0.5 * (a.0 + b.0), 0.5 * (a.1 + b.1), a.2 / na + b.2 / nb_, a.3 / na + b.3 / nb_)));
                        }
                    }
                }
            }
            for k in 1..4 {
                match pred[k] {
                    None => extra += 1, // computed: keep the exact sample
                    Some(c) => {
                        let i = idx(px, py, k);
                        match c {
                            Some((v, d, x, y)) if v.is_finite() && d > 0.0 && d.is_finite() => {
                                s.class[i] = Class::new(Kind::Escaped, Evidence::Heuristic);
                                s.nu.as_mut().unwrap()[i] = v;
                                s.de.as_mut().unwrap()[i] = d;
                                s.normal.as_mut().unwrap()[i] = Samples::angle(x, y);
                            }
                            _ => s.class[i] = Class::new(Kind::Unresolved, Evidence::Heuristic),
                        }
                    }
                }
            }
        }
        let (mut worst, mut wl, mut sum, mut mx) = (0.0f64, String::new(), 0.0f64, 0u8);
        for ((n, l), xi) in looks.iter().zip(&ex_img) {
            let img = l.shade_with(&hd, &s, &FILM);
            let mut c = 0usize;
            for (x, y) in xi.data.chunks(3).zip(img.data.chunks(3)) {
                let d = x.iter().zip(y).map(|(u, v)| u.abs_diff(*v)).max().unwrap();
                mx = mx.max(d);
                c += (d > 1) as usize;
            }
            let f = c as f64 / (w * h) as f64;
            sum += f;
            if f >= worst {
                worst = f;
                wl = n.clone();
            }
        }
        let label = if m < 0.0 { "ss1 (real sample copied)".to_string() } else { format!("corner m {m} per-sub") };
        println!(
            "  {label:26} eff {:.3} | >1 level mean {:.4}% worst {:.4}% ({wl}) | max {mx}",
            1.0 + extra as f64 / (w * h) as f64, 100.0 * sum / looks.len() as f64, 100.0 * worst
        );
    }
    Ok(())
}
