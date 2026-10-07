//! PROB-08: time the all-double deep-pixel pipeline on whole frames centred at the v0
//! minibrot nucleus C. Per pixel: PROB-07 biseries loops (Horner in u, with the v powers
//! folded in once per pixel), 23 perturbation steps against C's critical orbit, the
//! Koenigs jump along the repelling 2-cycle, then plain double iteration at fixed C.
//! Output per frame: `<out>/w<width>.bin` = class u8[n] (0 escaped, 1 not) then nu
//! f64[n] in fd's convention, row-major with fd's layout (x right, imaginary axis up),
//! plus one JSON line of timings on stdout.
//! MODE `perturb` is the fair lean baseline in the same code: plain double perturbation
//! against C's periodic reference orbit with Zhuoran rebasing (BLA does not beat plain
//! perturbation at these depths: METHOD.md, FIX-09).
//! Usage: koenigs-bench MODE CONSTS OUT_DIR NXxNY MAX_ITER THREADS RUNS WIDTH...   (MODE: koenigs | perturb)
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Tail patch polynomial length (tail_patches.py D).
const PATCH_D: usize = 16;

#[derive(Clone, Copy, Default)]
struct Cx(f64, f64);

impl Cx {
    #[inline(always)]
    fn add(self, o: Cx) -> Cx {
        Cx(self.0 + o.0, self.1 + o.1)
    }
    #[inline(always)]
    fn sub(self, o: Cx) -> Cx {
        Cx(self.0 - o.0, self.1 - o.1)
    }
    #[inline(always)]
    fn mul(self, o: Cx) -> Cx {
        Cx(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    #[inline(always)]
    fn scale(self, s: f64) -> Cx {
        Cx(self.0 * s, self.1 * s)
    }
    #[inline(always)]
    fn norm2(self) -> f64 {
        self.0 * self.0 + self.1 * self.1
    }
    #[inline(always)]
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
    #[inline(always)]
    fn div(self, o: Cx) -> Cx {
        let d = o.norm2();
        Cx((self.0 * o.0 + self.1 * o.1) / d, (self.1 * o.0 - self.0 * o.1) / d)
    }
}

/// Per-zone constants, computed once at high precision by koenigs_bench_consts.py.
#[derive(Default)]
struct Consts {
    c: Cx,
    period: u64,
    scale: f64,
    guard: f64,
    r0: f64,
    bias: Cx,
    alpha: Cx,
    rho: Cx,
    z24ma: Cx,
    orbit: Vec<Cx>,
    /// C's periodic reference orbit z_0 = 0 .. z_{P-1}.
    reference: Vec<Cx>,
    deg: usize,
    /// `bis[i][j]`: coefficient of u^i v^j.
    bis: Vec<Vec<Cx>>,
    /// `phi[k - 1]`: coefficient of h^k.
    phi: Vec<Cx>,
    /// `psi[k - 1]`: coefficient of w^k in psi = phi^-1 (series reversion, PROB-09).
    /// Empty: invert phi by Newton instead.
    psi: Vec<Cx>,
    /// Tail patch atlas (PROB-09, tail_patches.py): a quadtree over s = log w on the
    /// fundamental ring; each leaf holds n and a degree-D polynomial of f_C^n(A + psi(w))
    /// in t = (s - centre) / r. Empty: no patches.
    patch_nx: usize,
    /// (centre, first child or -1, leaf index or -1)
    nodes: Vec<(Cx, i64, i64)>,
    /// (centre, r, n or -1 if invalid, coefficients)
    leaves: Vec<(Cx, f64, i64, [Cx; PATCH_D])>,
}

fn load(path: &str) -> Consts {
    let text = fs::read_to_string(path).expect("read constants");
    let mut k = Consts::default();
    let mut raw = vec![];
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| f[i].parse::<f64>().expect("number");
        let cx = |i: usize| Cx(n(i), n(i + 1));
        match f[0] {
            "c" => k.c = cx(1),
            "period" => k.period = n(1) as u64,
            "scale" => k.scale = n(1),
            "guard" => k.guard = n(1),
            "r0" => k.r0 = n(1),
            "bias" => k.bias = cx(1),
            "alpha" => k.alpha = cx(1),
            "rho" => k.rho = cx(1),
            "z24_minus_alpha" => k.z24ma = cx(1),
            "orbit" => k.orbit.push(cx(2)),
            "ref" => k.reference.push(cx(2)),
            "biseries" => raw.push((n(1) as usize, n(2) as usize, cx(3))),
            "phi" => k.phi.push(cx(2)),
            "psi" => k.psi.push(cx(2)),
            "patch_root" => {
                k.patch_nx = n(3) as usize;
            }
            "node" => k.nodes.push((cx(1), n(3) as i64, n(4) as i64)),
            "leaf" => {
                let mut cf = [Cx::default(); PATCH_D];
                assert_eq!(f.len(), 5 + 2 * PATCH_D, "patch degree");
                for (i, c) in cf.iter_mut().enumerate() {
                    *c = cx(5 + 2 * i);
                }
                k.leaves.push((cx(1), n(3), n(4) as i64, cf));
            }
            other => panic!("unknown constant {other}"),
        }
    }
    k.deg = raw.iter().map(|&(i, j, _)| i + j).max().expect("biseries");
    assert!(k.deg < 8 && k.orbit.len() == 23 && k.reference.len() as u64 == k.period);
    k.bis = vec![vec![Cx::default(); k.deg + 1]; k.deg + 1];
    for (i, j, a) in raw {
        k.bis[i][j] = a;
    }
    k
}

/// Whether the pixel at offset `v` (in units of `scale`) escapes, and its nu
/// (`n + 1 - log2(log2|z|)`, as fd writes it).
#[inline]
fn pixel(k: &Consts, v: Cx, max_iter: u64, r2: f64) -> (bool, f64) {
    pixel_upto::<3>(k, v, max_iter, r2)
}

/// `pixel` stopped after stage STAGE (1 loops, 2 approach + jump, 3 finish), for
/// per-stage timing by subtraction (PROB-09). Stopped pixels report n as nu.
#[inline]
fn pixel_upto<const STAGE: u8>(k: &Consts, v: Cx, max_iter: u64, r2: f64) -> (bool, f64) {
    let mut b = [Cx::default(); 8];
    for (i, bi) in b.iter_mut().enumerate().take(k.deg + 1) {
        let mut s = Cx::default();
        for j in (0..=k.deg - i).rev() {
            s = s.mul(v).add(k.bis[i][j]);
        }
        *bi = s;
    }
    b[0] = b[0].add(k.bias);
    // Returns: u = (z_n - C) / scale.
    let mut u = v;
    let mut n: u64 = 1;
    while u.abs() * k.scale <= k.guard {
        if n + k.period > max_iter {
            return (false, 0.0);
        }
        let mut s = b[k.deg];
        for i in (0..k.deg).rev() {
            s = s.mul(u).add(b[i]);
        }
        u = s;
        n += k.period;
    }
    if STAGE == 1 {
        return (true, n as f64 + u.0);
    }
    // Exit tail at the fixed parameter C.
    let mut d = u.scale(k.scale);
    for z in &k.orbit {
        d = z.scale(2.0).mul(d).add(d.mul(d));
    }
    n += 23;
    let h0 = k.z24ma.add(d);
    let mut z = k.alpha.add(h0);
    if h0.abs() < k.r0 {
        let w0 = phi(k, h0);
        let lr = k.rho.abs().ln();
        let j = ((k.r0 / w0.abs()).ln() / lr).floor();
        if let Some((zp, m)) = patch(k, w0, j) {
            z = zp;
            n += 2 * j as u64 + m;
        } else if j > 0.0 {
            let (m, t) = ((j * lr).exp(), j * k.rho.1.atan2(k.rho.0));
            let w = w0.mul(Cx(m * t.cos(), m * t.sin()));
            let mut h = w;
            if !k.psi.is_empty() {
                let mut s = Cx::default();
                for c in k.psi.iter().rev() {
                    s = s.add(*c).mul(w);
                }
                h = s;
            }
            for _ in 0..if k.psi.is_empty() { 30 } else { 0 } {
                let (p, dp) = phi_d(k, h);
                let st = p.sub(w).div(dp);
                h = h.sub(st);
                if st.abs() <= h.abs() * 1e-16 {
                    break;
                }
            }
            z = k.alpha.add(h);
            n += 2 * j as u64;
        }
    }
    if STAGE == 2 {
        return (true, n as f64 + z.0);
    }
    loop {
        if n >= max_iter {
            return (false, 0.0);
        }
        z = z.mul(z).add(k.c);
        n += 1;
        let a = z.norm2();
        if a > r2 {
            return (true, n as f64 + 1.0 - (0.5 * a.log2()).log2());
        }
    }
}

/// Baseline: plain double perturbation, dz' = 2 Z dz + dz^2 + dc, rebasing to the start
/// of the reference when |z| < |dz| or the period wraps (z_P = 0 at the nucleus).
#[inline]
fn perturb(k: &Consts, v: Cx, max_iter: u64, r2: f64) -> (bool, f64) {
    let dc = v.scale(k.scale);
    let r = &k.reference;
    let (mut d, mut m) = (Cx::default(), 0usize);
    for n in 1..=max_iter {
        d = r[m].scale(2.0).mul(d).add(d.mul(d)).add(dc);
        m += 1;
        if m == r.len() {
            m = 0;
        }
        let z = r[m].add(d);
        let a = z.norm2();
        if a > r2 {
            return (true, n as f64 + 1.0 - (0.5 * a.log2()).log2());
        }
        if a < d.norm2() {
            d = z;
            m = 0;
        }
    }
    (false, 0.0)
}

/// The tail patch for the jumped point w0 * rho^j: f_C^m(A + psi(w)) and m.
#[inline]
fn patch(k: &Consts, w0: Cx, j: f64) -> Option<(Cx, u64)> {
    if k.nodes.is_empty() || j < 0.0 {
        return None;
    }
    let tau = std::f64::consts::TAU;
    let re = w0.abs().ln() + j * k.rho.abs().ln();
    let im = (w0.1.atan2(w0.0) + j * k.rho.1.atan2(k.rho.0)).rem_euclid(tau);
    let mut node = ((im / (tau / k.patch_nx as f64)) as usize).min(k.patch_nx - 1);
    while k.nodes[node].1 >= 0 {
        let c = k.nodes[node].0;
        node = k.nodes[node].1 as usize + 2 * usize::from(re >= c.0) + usize::from(im >= c.1);
    }
    let (c, r, m, cf) = &k.leaves[k.nodes[node].2 as usize];
    if *m < 0 {
        return None;
    }
    let t = Cx((re - c.0) / r, (im - c.1) / r);
    let mut z = Cx::default();
    for a in cf.iter().rev() {
        z = z.mul(t).add(*a);
    }
    Some((z, *m as u64))
}

#[inline]
fn phi(k: &Consts, h: Cx) -> Cx {
    let mut s = Cx::default();
    for c in k.phi.iter().rev() {
        s = s.add(*c).mul(h);
    }
    s
}

#[inline]
fn phi_d(k: &Consts, h: Cx) -> (Cx, Cx) {
    let (mut s, mut d) = (Cx::default(), Cx::default());
    for (i, c) in k.phi.iter().enumerate().rev() {
        s = s.add(*c).mul(h);
        d = d.mul(h).add(c.scale((i + 1) as f64));
    }
    (s, d)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    assert!(a.len() >= 9, "usage: koenigs-bench MODE CONSTS OUT_DIR NXxNY MAX_ITER THREADS RUNS WIDTH...");
    let f: fn(&Consts, Cx, u64, f64) -> (bool, f64) = match a[1].as_str() {
        "koenigs" => pixel,
        "koenigs-loops" => pixel_upto::<1>,
        "koenigs-jump" => pixel_upto::<2>,
        "perturb" => perturb,
        m => panic!("unknown mode {m}"),
    };
    let a = &a[1..];
    let k = load(&a[1]);
    let out = &a[2];
    let (nx, ny) = a[3].split_once('x').map(|(x, y)| (x.parse::<usize>().unwrap(), y.parse::<usize>().unwrap())).unwrap();
    let max_iter: u64 = a[4].parse().unwrap();
    let threads: usize = a[5].parse().unwrap();
    let runs: usize = a[6].parse().unwrap();
    let r2 = 1e20;
    fs::create_dir_all(out).unwrap();
    for width_s in &a[7..] {
        let width: f64 = width_s.parse().unwrap();
        let h = width / nx as f64;
        let n = nx * ny;
        let (mut class, mut nu) = (vec![0u8; n], vec![0f64; n]);
        let mut times = vec![];
        for _ in 0..runs {
            let next = AtomicUsize::new(0);
            let t = Instant::now();
            let rows: Vec<(usize, Vec<u8>, Vec<f64>)> = std::thread::scope(|s| {
                let hs: Vec<_> = (0..threads)
                    .map(|_| {
                        s.spawn(|| {
                            let mut mine = vec![];
                            loop {
                                let j = next.fetch_add(1, Ordering::Relaxed);
                                if j >= ny {
                                    break;
                                }
                                let (mut cl, mut nv) = (vec![0u8; nx], vec![0f64; nx]);
                                let y = -(j as f64 + 0.5 - ny as f64 / 2.0) * h;
                                for i in 0..nx {
                                    let x = (i as f64 + 0.5 - nx as f64 / 2.0) * h;
                                    let (esc, v) = f(&k, Cx(x / k.scale, y / k.scale), max_iter, r2);
                                    cl[i] = u8::from(!esc);
                                    nv[i] = v;
                                }
                                mine.push((j, cl, nv));
                            }
                            mine
                        })
                    })
                    .collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            times.push(t.elapsed().as_secs_f64());
            for (j, cl, nv) in rows {
                class[j * nx..(j + 1) * nx].copy_from_slice(&cl);
                nu[j * nx..(j + 1) * nx].copy_from_slice(&nv);
            }
        }
        let mut bytes = class.clone();
        for x in &nu {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
        fs::write(format!("{out}/w{width_s}.bin"), bytes).unwrap();
        let best = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let escaped = class.iter().filter(|&&c| c == 0).count();
        println!(
            "{{\"mode\":\"{}\",\"width\":\"{width_s}\",\"nx\":{nx},\"ny\":{ny},\"threads\":{threads},\"runs\":{runs},\"best_seconds\":{best},\"all_seconds\":{times:?},\"escaped\":{escaped},\"pixels\":{n}}}",
            a[0]
        );
    }
}
