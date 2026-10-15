//! Score jet-AA against the exact ss 2 render (DEC-29) and the 4x4 truth, shaded with the
//! project's looks (fd-shade, film appearance: aa on, unresolved drawn as interior, no dither).
//! usage: score DUMP [ORDERS] [TAUS]
//!   ORDERS: comma list of jet orders (default 1,2,3); TAUS: comma list of DE flag thresholds in
//!   output px (default 0,0.5,1,2,3,5). A pixel is flagged (rendered with its 4 true ss 2
//!   samples) when its centre DE < tau, or its centre is unresolved and some 8-neighbour
//!   centre escaped. Unflagged pixels take 4 virtual samples from the centre's jet; an
//!   unresolved centre with all 8 neighbours unresolved gives 4 unresolved samples.
//!   Effective samples per pixel = 1 + 4 * flagged share.
use fd_samples::{Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View, MINOR};
use fd_shade::{Appearance, Pass};
use jetaa::*;

pub fn header(nx: usize, ny: usize, ss: u32) -> Header {
    Header {
        minor: MINOR,
        columns: ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]),
        nx: nx as u32,
        ny: ny as u32,
        ss,
        max_iter: 100_000,
        escape_radius: 1e10,
        view: View { center_re: "0".into(), center_im: "0".into(), width: "1".into(), rotation: 0.0 },
        kernel: "jetaa".into(),
    }
}

fn put(s: &mut Samples, i: usize, c: Option<(f64, f32, f64, f64)>) {
    match c {
        Some((nu, de, x, y)) if nu.is_finite() && de.is_finite() && de > 0.0 => {
            s.class[i] = Class::new(Kind::Escaped, Evidence::Heuristic);
            s.nu.as_mut().unwrap()[i] = nu;
            s.de.as_mut().unwrap()[i] = de;
            s.normal.as_mut().unwrap()[i] = Samples::angle(x, y);
        }
        Some(_) => {
            // the jet broke down (ln|z| <= 0 or DE <= 0): mark it so it is visible
            s.class[i] = Class::new(Kind::Unresolved, Evidence::Heuristic);
        }
        None => s.class[i] = Class::new(Kind::Unresolved, Evidence::Heuristic),
    }
}

pub fn looks() -> Vec<(String, Box<dyn Pass>)> {
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
}

pub const FILM: Appearance = Appearance { aa: true, unresolved_interior: true, ..Appearance::STILL };

/// Exact ss 2 samples.
pub fn exact(d: &Dump) -> Samples {
    let mut s = Samples::alloc(4 * d.w * d.h, header(1, 1, 1).columns);
    for p in 0..d.w * d.h {
        let r = d.rec(p);
        let (px, py) = (p % d.w, p / d.w);
        for k in 0..4 {
            let i = (2 * py + k / 2) * 2 * d.w + 2 * px + k % 2;
            put(&mut s, i, columns(&r[CENTRE + 5 * k..CENTRE + 5 * k + 5]));
        }
    }
    s
}

/// 4x4 truth samples.
pub fn truth(d: &Dump) -> Samples {
    let mut s = Samples::alloc(16 * d.w * d.h, header(1, 1, 1).columns);
    for p in 0..d.w * d.h {
        let r = d.rec(p);
        let (px, py) = (p % d.w, p / d.w);
        for k in 0..16 {
            let i = (4 * py + k / 4) * 4 * d.w + 4 * px + k % 4;
            let b = CENTRE + SS2 + 5 * k;
            put(&mut s, i, columns(&r[b..b + 5]));
        }
    }
    s
}

/// ss 1 from the centre samples.
pub fn single(d: &Dump) -> Samples {
    let mut s = Samples::alloc(d.w * d.h, header(1, 1, 1).columns);
    for p in 0..d.w * d.h {
        put(&mut s, p, columns(&d.rec(p)[..CENTRE]));
    }
    s
}

/// Is pixel p flagged at threshold tau?
pub fn flagged(d: &Dump, p: usize, tau: f32) -> bool {
    let r = d.rec(p);
    match columns(&r[..CENTRE]) {
        Some(c) => c.1 < tau,
        None => {
            let (px, py) = ((p % d.w) as i64, (p / d.w) as i64);
            for dy in -1..=1i64 {
                for dx in -1..=1i64 {
                    let (x, y) = (px + dx, py + dy);
                    if x < 0 || y < 0 || x >= d.w as i64 || y >= d.h as i64 {
                        continue;
                    }
                    if d.rec((y * d.w as i64 + x) as usize)[0] != 0.0 {
                        return true;
                    }
                }
            }
            false
        }
    }
}

/// The jet of pixel p: exact (`nbr` false) or its higher terms from the 4 neighbours.
pub fn pixel_jet(d: &Dump, p: usize, nbr: bool) -> Option<Jet> {
    let r = d.rec(p);
    if !nbr {
        return Jet::from_raw(&r[..CENTRE]);
    }
    let (px, py) = (p % d.w, p / d.w);
    if px == 0 || py == 0 || px + 1 == d.w || py + 1 == d.h {
        return None;
    }
    let c = |q: usize| &d.rec(q)[..CENTRE];
    jet_from_neighbours(c(p), [c(p + 1), c(p - 1), c(p - d.w), c(p + d.w)])
}

/// Jet ss 2 samples, flagged pixels from the true subsamples. Returns (samples, flagged share).
/// `order` >= 10 means order - 10 with the neighbour jet.
/// Also returns the work relative to the exact ss 2 render: (samples, iterations), counted
/// over pixels off the crop border (a real frame gives border pixels their neighbours with
/// a 1 px apron); unresolved samples count as `MAXIT` iterations (env, default 50000).
pub fn jet(d: &Dump, order: u32, tau: f32) -> (Samples, f64, f64) {
    let (order, nbr) = if order >= 10 { (order - 10, true) } else { (order, false) };
    let mut s = Samples::alloc(4 * d.w * d.h, header(1, 1, 1).columns);
    let maxit: f64 = std::env::var("MAXIT").ok().and_then(|v| v.parse().ok()).unwrap_or(50000.0);
    let it = |x: f64| if x == 0.0 { maxit } else { x };
    let (mut nf, mut ni, mut wj, mut wx) = (0usize, 0usize, 0.0f64, 0.0f64);
    for p in 0..d.w * d.h {
        let r = d.rec(p);
        let (px, py) = (p % d.w, p / d.w);
        let border = px == 0 || py == 0 || px + 1 == d.w || py + 1 == d.h;
        let j = pixel_jet(d, p, nbr);
        let f = border || flagged(d, p, tau) || (r[0] != 0.0 && j.is_none());
        if !border {
            ni += 1;
            nf += f as usize;
            let sub: f64 = (0..4).map(|k| it(r[CENTRE + 5 * k])).sum();
            wx += sub;
            wj += it(r[0]) + if f { sub } else { 0.0 };
        }
        for k in 0..4 {
            let i = (2 * py + k / 2) * 2 * d.w + 2 * px + k % 2;
            let (dx, dy) = ((k % 2) as f64 * 0.5 - 0.25, (k / 2) as f64 * 0.5 - 0.25);
            let c = if f {
                columns(&r[CENTRE + 5 * k..CENTRE + 5 * k + 5])
            } else {
                j.map(|j| j.at(dx, dy, order))
            };
            put(&mut s, i, c);
        }
    }
    (s, 1.0 + 4.0 * nf as f64 / ni as f64, 4.0 * wj / wx)
}

/// Per-pixel band: min DE over the exact ss 2 subsamples (0 if any is unresolved).
pub fn bands(d: &Dump) -> Vec<f32> {
    (0..d.w * d.h)
        .map(|p| {
            let r = d.rec(p);
            (0..4)
                .map(|k| columns(&r[CENTRE + 5 * k..CENTRE + 5 * k + 5]).map_or(0.0, |c| c.1))
                .fold(f32::INFINITY, f32::min)
        })
        .collect()
}

pub const BANDS: [f32; 6] = [0.0, 0.1, 0.3, 1.0, 3.0, f32::INFINITY];

/// Compare two images: (share of pixels > 1 level, share > 4, max, per-band counts > 1).
pub fn cmp(a: &fd_shade::Rgb8, b: &fd_shade::Rgb8, band: &[f32]) -> (f64, f64, u8, [usize; 5]) {
    let (mut n1, mut n4, mut mx) = (0usize, 0usize, 0u8);
    let mut nb = [0usize; 5];
    for (p, (x, y)) in a.data.chunks(3).zip(b.data.chunks(3)).enumerate() {
        let d = x.iter().zip(y).map(|(u, v)| u.abs_diff(*v)).max().unwrap();
        mx = mx.max(d);
        if d > 1 {
            n1 += 1;
            nb[(0..5).find(|&k| band[p] < BANDS[k + 1]).unwrap_or(4)] += 1;
        }
        if d > 4 {
            n4 += 1;
        }
    }
    let n = (a.w * a.h) as f64;
    (n1 as f64 / n, n4 as f64 / n, mx, nb)
}

/// Virtual ss 4: 16 subsamples from the pixel's jet; flagged pixels take the 16 true ones.
/// Returns (samples, effective samples per pixel = 1 + 16 * flagged share).
pub fn jet4(d: &Dump, order: u32, tau: f32) -> (Samples, f64) {
    let (order, nbr) = if order >= 10 { (order - 10, true) } else { (order, false) };
    let mut s = Samples::alloc(16 * d.w * d.h, header(1, 1, 1).columns);
    let (mut nf, mut ni) = (0usize, 0usize);
    for p in 0..d.w * d.h {
        let r = d.rec(p);
        let (px, py) = (p % d.w, p / d.w);
        let border = px == 0 || py == 0 || px + 1 == d.w || py + 1 == d.h;
        let j = pixel_jet(d, p, nbr);
        let f = border || flagged(d, p, tau) || (r[0] != 0.0 && j.is_none());
        if !border {
            ni += 1;
            nf += f as usize;
        }
        for k in 0..16 {
            let i = (4 * py + k / 4) * 4 * d.w + 4 * px + k % 4;
            let (dx, dy) = ((k % 4) as f64 * 0.25 - 0.375, (k / 4) as f64 * 0.25 - 0.375);
            let b = CENTRE + SS2 + 5 * k;
            let c = if f { columns(&r[b..b + 5]) } else { j.map(|j| j.at(dx, dy, order)) };
            put(&mut s, i, c);
        }
    }
    (s, 1.0 + 16.0 * nf as f64 / ni as f64)
}

/// Corner scheme: each pixel's one real sample is its top-left ss 2 subsample (-1/4, -1/4);
/// its jet comes from the neighbour stencil over the neighbours' real samples. The other
/// three subsamples are predicted twice: from the pixel's own real sample (offsets
/// (1/2, 0), (0, 1/2), (1/2, 1/2)) and from the right, lower and diagonal neighbour's real
/// sample (the mirrored offsets). A subsample is accepted when the two predictions agree
/// (|dnu| < m 1e-3, |d ln de| < m 1e-2, |d angle| < m 3e-3 rad) and the own real sample has
/// de >= tau0; it then takes their mean. Otherwise it is computed (true subsample).
/// `per_sub`: only the rejected subsamples are computed; else any rejection computes all 3.
/// Returns (samples, effective samples per pixel, work vs exact ss 2).
pub fn corner(d: &Dump, m: f64, tau0: f32, per_sub: bool) -> (Samples, f64, f64) {
    let maxit: f64 = std::env::var("MAXIT").ok().and_then(|v| v.parse().ok()).unwrap_or(50000.0);
    let it = |x: f64| if x == 0.0 { maxit } else { x };
    // QUANT=1: rebuild each real sample's (n, z, dz/du) from its stored columns only
    // (nu f64, de f32, normal u16), as a late pass over an .fds file would see it.
    let quant = std::env::var("QUANT").map_or(false, |v| v == "1");
    let reals: Vec<[f64; 5]> = (0..d.w * d.h)
        .map(|q| {
            let r = &d.rec(q)[CENTRE..CENTRE + 5];
            let mut o = [0.0f64; 5];
            o.copy_from_slice(r);
            if quant && r[0] != 0.0 {
                let (nu, de, x, y) = columns(r).unwrap();
                let n = nu.floor();
                let l = std::f64::consts::LN_2 * (n + 1.0 - nu).exp2(); // ln|z|
                let (sn, cs) = (Samples::unit(Samples::angle(x, y)).1 as f64, Samples::unit(Samples::angle(x, y)).0 as f64);
                let aw = 2.0 * l / de as f64; // |w|
                let ez = l.exp();
                o = [n, ez, 0.0, aw * cs * ez, aw * sn * ez];
            }
            o
        })
        .collect();
    let real = |q: usize| &reals[q][..];
    let sub = |q: usize, k: usize| &d.rec(q)[CENTRE + 5 * k..CENTRE + 5 * k + 5];
    let inner = |q: usize| {
        let (x, y) = (q % d.w, q / d.w);
        x > 0 && y > 0 && x + 1 < d.w && y + 1 < d.h
    };
    let jet = |q: usize| -> Option<Jet> {
        if !inner(q) {
            return None;
        }
        // pad the 5-value records to the 9-value raw layout (no d2, d3)
        let pad = |r: &[f64]| {
            let mut a = [0.0f64; 9];
            a[..5].copy_from_slice(r);
            a
        };
        let (c, rt, lf, up, dn) = (pad(real(q)), pad(real(q + 1)), pad(real(q - 1)), pad(real(q - d.w)), pad(real(q + d.w)));
        jet_from_neighbours(&c, [&rt, &lf, &up, &dn])
    };
    let mut s = Samples::alloc(4 * d.w * d.h, header(1, 1, 1).columns);
    let (mut ni, mut extra, mut wj, mut wx) = (0usize, 0usize, 0.0f64, 0.0f64);
    let ang = |x: f64, y: f64| y.atan2(x);
    for p in 0..d.w * d.h {
        let (px, py) = (p % d.w, p / d.w);
        let border = px + 2 >= d.w || py + 2 >= d.h || px == 0 || py == 0;
        let r0 = real(p);
        // per subsample k = 1, 2, 3: Some(columns) if accepted
        let mut pred: [Option<Option<(f64, f32, f64, f64)>>; 4] = [None; 4];
        if !border {
            let own = jet(p);
            let nb = [p + 1, p + d.w, p + d.w + 1];
            if r0[0] == 0.0 {
                // unresolved real sample: predict unresolved only if the three neighbours'
                // real samples and the 4-neighbourhood are all unresolved too
                let all = [p + 1, p + d.w, p + d.w + 1, p - 1, p - d.w].iter().all(|&q| real(q)[0] == 0.0);
                if all {
                    for k in 1..4 {
                        pred[k] = Some(None);
                    }
                }
            } else if let Some(j) = own {
                let de0 = columns(r0).unwrap().1;
                for k in 1..4 {
                    if de0 < tau0 {
                        continue;
                    }
                    let (ox, oy) = ((k % 2) as f64 * 0.5, (k / 2) as f64 * 0.5);
                    let Some(jn) = jet(nb[k - 1]) else { continue };
                    let a = j.at(ox, oy, 3);
                    let b = jn.at(ox - (k % 2) as f64, oy - (k / 2) as f64, 3);
                    let ok = (a.0 - b.0).abs() < m * 1e-3
                        && ((a.1 / b.1) as f64).ln().abs() < m * 1e-2
                        && {
                            let da = (ang(a.2, a.3) - ang(b.2, b.3) + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
                            da.abs() < m * 3e-3
                        };
                    if ok {
                        // mean of the two predictions (normals as unit vectors)
                        let (na, nb_) = (a.2.hypot(a.3), b.2.hypot(b.3));
                        pred[k] = Some(Some((
                            0.5 * (a.0 + b.0),
                            0.5 * (a.1 + b.1),
                            a.2 / na + b.2 / nb_,
                            a.3 / na + b.3 / nb_,
                        )));
                    }
                }
            }
        }
        let rejected: Vec<usize> = (1..4).filter(|&k| pred[k].is_none()).collect();
        let compute_all = !per_sub && !rejected.is_empty();
        for k in 0..4 {
            let i = (2 * py + k / 2) * 2 * d.w + 2 * px + k % 2;
            let c = if k == 0 || border || compute_all || pred[k].is_none() {
                columns(sub(p, k))
            } else {
                pred[k].unwrap()
            };
            put(&mut s, i, c);
        }
        if !border {
            ni += 1;
            let tot: f64 = (0..4).map(|k| it(sub(p, k)[0])).sum();
            wx += tot;
            wj += it(r0[0]);
            let ks: Vec<usize> = if compute_all { vec![1, 2, 3] } else { rejected.clone() };
            extra += ks.len();
            wj += ks.iter().map(|&k| it(sub(p, k)[0])).sum::<f64>();
        }
    }
    (s, 1.0 + extra as f64 / ni as f64, 4.0 * wj / wx)
}

/// Flicker on a zoom sequence (frames in order, same crop): per method, the per-frame
/// DEC-29 share, and frame-to-frame change against the exact film. For consecutive frames,
/// x = max channel |X(t+1) - X(t)| (exact film), j = the same for the method's film, per
/// pixel: reports mean x, mean j, and the shares of pixel-pairs with j - x > 1 and > 4
/// levels (change the exact film does not have), and with |(J-X)(t+1) - (J-X)(t)| > 1.
fn seq(paths: &[String]) -> Result<(), String> {
    let looks = looks();
    let ds: Vec<Dump> = paths.iter().map(|p| Dump::load(p)).collect::<Result<_, _>>()?;
    let (w, h) = (ds[0].w, ds[0].h);
    let h2 = header(2 * w, 2 * h, 2);
    type Method = (&'static str, Box<dyn Fn(&Dump) -> Samples>);
    let methods: Vec<Method> = vec![
        ("centre n3 tau 3", Box::new(|d: &Dump| jet(d, 13, 3.0).0)),
        ("centre n3 tau 5", Box::new(|d: &Dump| jet(d, 13, 5.0).0)),
        ("centre o3 tau 3", Box::new(|d: &Dump| jet(d, 3, 3.0).0)),
        ("corner m 3 per-sub", Box::new(|d: &Dump| corner(d, 3.0, 0.0, true).0)),
        ("corner m 10 per-sub", Box::new(|d: &Dump| corner(d, 10.0, 0.0, true).0)),
        ("ss1 (no AA)", Box::new(|d: &Dump| {
            // ss 1 shown on the ss 2 grid: each centre copied 4 times
            let c = single(d);
            let mut s = Samples::alloc(4 * d.w * d.h, header(1, 1, 1).columns);
            for p in 0..d.w * d.h {
                let (px, py) = (p % d.w, p / d.w);
                for k in 0..4 {
                    let i = (2 * py + k / 2) * 2 * d.w + 2 * px + k % 2;
                    s.class[i] = c.class[p];
                    s.nu.as_mut().unwrap()[i] = c.nu.as_ref().unwrap()[p];
                    s.de.as_mut().unwrap()[i] = c.de.as_ref().unwrap()[p];
                    s.normal.as_mut().unwrap()[i] = c.normal.as_ref().unwrap()[p];
                }
            }
            s
        })),
    ];
    let maxd = |a: &[u8], b: &[u8]| a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap() as i32;
    let ex: Vec<Vec<fd_shade::Rgb8>> = ds.iter().map(|d| { let s = exact(d); looks.iter().map(|(_, l)| l.shade_with(&h2, &s, &FILM)).collect() }).collect();
    println!("== sequence {} frames {}x{}", ds.len(), w, h);
    for (name, f) in &methods {
        let im: Vec<Vec<fd_shade::Rgb8>> = ds.iter().map(|d| { let s = f(d); looks.iter().map(|(_, l)| l.shade_with(&h2, &s, &FILM)).collect() }).collect();
        let (mut frame1, mut worst1) = (0.0f64, 0.0f64);
        let (mut sx, mut sj, mut n, mut add1, mut add4, mut e1) = (0.0f64, 0.0f64, 0usize, 0usize, 0usize, 0usize);
        for l in 0..looks.len() {
            for t in 0..ds.len() {
                let (x, j) = (&ex[t][l].data, &im[t][l].data);
                let c = x.chunks(3).zip(j.chunks(3)).filter(|(a, b)| maxd(a, b) > 1).count() as f64 / (w * h) as f64;
                frame1 += c;
                worst1 = worst1.max(c);
                if t + 1 < ds.len() {
                    let (x2, j2) = (&ex[t + 1][l].data, &im[t + 1][l].data);
                    for q in 0..w * h {
                        let r = 3 * q..3 * q + 3;
                        let dx = maxd(&x[r.clone()], &x2[r.clone()]);
                        let dj = maxd(&j[r.clone()], &j2[r.clone()]);
                        sx += dx as f64;
                        sj += dj as f64;
                        n += 1;
                        add1 += (dj - dx > 1) as usize;
                        add4 += (dj - dx > 4) as usize;
                        let ee = (0..3).map(|c| ((j2[3 * q + c] as i32 - x2[3 * q + c] as i32) - (j[3 * q + c] as i32 - x[3 * q + c] as i32)).abs()).max().unwrap();
                        e1 += (ee > 1) as usize;
                    }
                }
            }
        }
        let frames = (looks.len() * ds.len()) as f64;
        println!(
            "  {name:22} frame >1lvl mean {:.4}% worst {:.4}% | mean |dX| {:.3} mean |dJ| {:.3} | dJ-dX >1: {:.4}% >4: {:.4}% | |d(J-X)| >1: {:.4}%",
            100.0 * frame1 / frames, 100.0 * worst1, sx / n as f64, sj / n as f64,
            100.0 * add1 as f64 / n as f64, 100.0 * add4 as f64 / n as f64, 100.0 * e1 as f64 / n as f64
        );
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    if a[1] == "--seq" {
        return seq(&a[2..]);
    }
    let d = Dump::load(&a[1])?;
    // "n3" = third order from the neighbour stencil (coded as 13)
    let orders: Vec<u32> = a.get(2).map_or("1,2,3,n2,n3", |s| s).split(',').map(|x| x.strip_prefix('n').map_or_else(|| x.parse().unwrap(), |y| 10 + y.parse::<u32>().unwrap())).collect();
    let taus: Vec<f32> = a.get(3).map_or("0,0.5,1,2,3,5", |s| s).split(',').map(|x| x.parse().unwrap()).collect();
    let looks = looks();
    let (h1, h2, h4) = (header(d.w, d.h, 1), header(2 * d.w, 2 * d.h, 2), header(4 * d.w, 4 * d.h, 4));
    let band = bands(&d);
    let mut bc = [0usize; 5];
    for &b in &band {
        bc[(0..5).find(|&k| b < BANDS[k + 1]).unwrap_or(4)] += 1;
    }
    println!("== {} {}x{} pixels by min ss2 DE: <0.1 {} | 0.1-0.3 {} | 0.3-1 {} | 1-3 {} | >=3 {}", a[1], d.w, d.h, bc[0], bc[1], bc[2], bc[3], bc[4]);
    let ex = exact(&d);
    let ex_img: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h2, &ex, &FILM)).collect();
    let report = |label: &str, imgs: &[fd_shade::Rgb8], refs: &[fd_shade::Rgb8], extra: &str| {
        let mut worst = (0.0f64, String::new());
        let (mut s1, mut s4, mut mx) = (0.0, 0.0, 0u8);
        let mut nb = [0usize; 5];
        for ((n, _), (x, y)) in looks.iter().zip(imgs.iter().zip(refs)) {
            let (f1, f4, m, b) = cmp(x, y, &band);
            if f1 >= worst.0 {
                worst = (f1, n.clone());
            }
            s1 += f1;
            s4 += f4;
            mx = mx.max(m);
            for k in 0..5 {
                nb[k] += b[k];
            }
        }
        let k = looks.len() as f64;
        println!(
            "  {label:34} {extra} >1lvl mean {:.4}% worst {:.4}% ({}) | >4lvl mean {:.4}% | max {mx} | >1 by band (pixel-looks) {:?}",
            100.0 * s1 / k, 100.0 * worst.0, worst.1, 100.0 * s4 / k, nb
        );
    };
    if let Ok(spec) = std::env::var("CORNER") {
        // CORNER="m1,m2;tau1,tau2" sweeps the consistency multiplier and the de floor
        let (ms, ts) = spec.split_once(';').unwrap_or((&spec, "0"));
        for m in ms.split(',').map(|x| x.parse::<f64>().unwrap()) {
            for t in ts.split(',').map(|x| x.parse::<f32>().unwrap()) {
                for per in [true, false] {
                    let (j, f, w) = corner(&d, m, t, per);
                    let ji: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h2, &j, &FILM)).collect();
                    let lab = format!("corner m {m} tau0 {t} {}", if per { "per-sub" } else { "per-pixel" });
                    report(&lab, &ji, &ex_img, &format!("eff {f:.2} work {w:.2}"));
                }
            }
        }
        return Ok(());
    }
    if d.has_ss4 {
        let tr = truth(&d);
        let tr_img: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h4, &tr, &FILM)).collect();
        report("exact ss2 vs truth 4x4", &ex_img, &tr_img, "eff 4.00");
        // the same exact ss 2 render with the camera moved by (-1/8, -1/8) px: its samples
        // are the 4x4 truth samples at x, y in {-3/8, +1/8}
        let mut sh = Samples::alloc(4 * d.w * d.h, header(1, 1, 1).columns);
        for p in 0..d.w * d.h {
            let r = d.rec(p);
            let (px, py) = (p % d.w, p / d.w);
            for k in 0..4 {
                let i = (2 * py + k / 2) * 2 * d.w + 2 * px + k % 2;
                let t = (k / 2) * 2 * 4 + (k % 2) * 2; // rows 0, 2 and columns 0, 2 of 4x4
                let b = CENTRE + SS2 + 5 * t;
                put(&mut sh, i, columns(&r[b..b + 5]));
            }
        }
        let sh_img: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h2, &sh, &FILM)).collect();
        report("exact ss2 shifted 1/8 px vs exact", &sh_img, &ex_img, "eff 4.00");
        // a different exact 4-sample pattern with the same centroid: the rotated grid
        // (-3/8,-1/8), (1/8,-3/8), (3/8,1/8), (-1/8,3/8) on the 4x4 truth lattice
        let mut rg = Samples::alloc(4 * d.w * d.h, header(1, 1, 1).columns);
        for p in 0..d.w * d.h {
            let r = d.rec(p);
            let (px, py) = (p % d.w, p / d.w);
            for (k, t) in [4usize, 2, 11, 13].iter().enumerate() {
                let i = (2 * py + k / 2) * 2 * d.w + 2 * px + k % 2;
                let b = CENTRE + SS2 + 5 * t;
                put(&mut rg, i, columns(&r[b..b + 5]));
            }
        }
        let rg_img: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h2, &rg, &FILM)).collect();
        report("exact RGSS-4 vs exact ss2", &rg_img, &ex_img, "eff 4.00");
        let si = single(&d);
        let si_img: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h1, &si, &FILM)).collect();
        report("ss1 (centre) vs truth 4x4", &si_img, &tr_img, "eff 1.00");
        report("ss1 (centre) vs exact ss2", &si_img, &ex_img, "eff 1.00");
        if std::env::var("SS4").map_or(false, |v| v == "1") {
            for &o in &orders {
                for &t in &taus {
                    let (j, f) = jet4(&d, o, t);
                    let ji: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h4, &j, &FILM)).collect();
                    let on = if o >= 10 { format!("n{}", o - 10) } else { format!("o{o}") };
                    report(&format!("virtual ss4 {on} tau {t} vs truth 4x4"), &ji, &tr_img, &format!("eff {f:.2}"));
                }
            }
            return Ok(());
        }
        for &o in &orders {
            for &t in &taus {
                let (j, f, w) = jet(&d, o, t);
                let ji: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h2, &j, &FILM)).collect();
                let e = format!("eff {f:.2} work {w:.2}");
                let on = if o >= 10 { format!("n{}", o - 10) } else { format!("o{o}") };
                report(&format!("jet {on} tau {t} vs exact ss2"), &ji, &ex_img, &e);
                report(&format!("jet {on} tau {t} vs truth 4x4"), &ji, &tr_img, &e);
            }
        }
    } else {
        for &o in &orders {
            for &t in &taus {
                let (j, f, w) = jet(&d, o, t);
                let ji: Vec<_> = looks.iter().map(|(_, l)| l.shade_with(&h2, &j, &FILM)).collect();
                let on = if o >= 10 { format!("n{}", o - 10) } else { format!("o{o}") };
                report(&format!("jet {on} tau {t} vs exact ss2"), &ji, &ex_img, &format!("eff {f:.2} work {w:.2}"));
            }
        }
    }
    Ok(())
}
