//! PROB-14: co-moving 2-cycle Koenigs jump for the six v0 mid-band frames.
//! The shortcut is heuristic within |dc| <= 1e-6 and |q| <= 0.03; invalid
//! steps fall back to full perturbation against the high-precision zone orbit.
//! DEC-17: only whole-frame fd compare results can qualify this for use in fd.
use super::Cx;
use fd_samples::{write, Class, Column, ColumnSet, Evidence, Header, Kind, Samples, View, MINOR};
use std::fs::{self, File};
use std::io::BufWriter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const N: usize = 18;
const APPROACH: usize = 24;
const R2: f64 = 1e20;

struct Mid {
    re: String,
    im: String,
    c: Cx,
    z: Vec<Cx>,
    reference: Vec<Cx>,
    u: Cx,
    s: Cx,
    p: Cx,
    sign: f64,
    lambda: Cx,
    k: [Cx; N + 1],
    kp: [Cx; N + 1],
    powers: Vec<Cx>,
}

fn load(path: &str) -> Mid {
    let mut m = Mid {
        re: String::new(), im: String::new(), c: Cx::default(),
        z: Vec::new(), reference: Vec::new(), u: Cx::default(), s: Cx::default(),
        p: Cx::default(), sign: 0.0, lambda: Cx::default(),
        k: [Cx::default(); N + 1], kp: [Cx::default(); N + 1], powers: Vec::new(),
    };
    for line in fs::read_to_string(path).expect("mid constants").lines() {
        if line.is_empty() || line.starts_with('#') { continue; }
        let a: Vec<&str> = line.split_whitespace().collect();
        let num = |i: usize| a[i].parse::<f64>().expect("mid constant number");
        let cx = |i: usize| Cx(num(i), num(i + 1));
        match a[0] {
            "c_exact" => { m.re = a[1].into(); m.im = a[2].into(); }
            "c" => m.c = cx(1),
            "z" => { assert_eq!(num(1) as usize, m.z.len()); m.z.push(cx(2)); }
            "ref" => { assert_eq!(num(1) as usize, m.reference.len()); m.reference.push(cx(2)); }
            "u" => m.u = cx(1),
            "s" => m.s = cx(1),
            "p" => m.p = cx(1),
            "sign" => m.sign = num(1),
            "lambda" => m.lambda = cx(1),
            "k" => m.k[num(1) as usize] = cx(2),
            "kp" => m.kp[num(1) as usize] = cx(2),
            "t" => { assert_eq!(num(1) as usize, m.powers.len()); m.powers.push(cx(2)); }
            x => panic!("unknown mid constant {x}"),
        }
    }
    assert_eq!(m.z.len(), APPROACH + 1);
    assert_eq!(m.reference.len(), 764);
    assert!(m.powers.len() > 400 && m.sign.abs() == 1.0);
    assert!(!m.re.is_empty() && !m.im.is_empty());
    m
}

#[derive(Clone, Copy)]
struct ResultPixel {
    escaped: bool,
    nu: f64,
    de: f32,
    normal: u16,
    fallback: bool,
}

impl ResultPixel {
    fn unresolved(fallback: bool) -> Self {
        Self { escaped: false, nu: 0.0, de: 0.0, normal: 0, fallback }
    }

    fn escaped(n: u64, z: Cx, dz: Cx, h: f64, fallback: bool) -> Self {
        let a = z.norm2();
        let d = 2.0 * a.sqrt() * (0.5 * a.ln()) / (dz.abs() * h);
        let g = z.div(dz);
        Self {
            escaped: true,
            nu: n as f64 + 1.0 - (0.5 * a.log2()).log2(),
            de: d as f32,
            normal: Samples::angle(g.0, -g.1),
            fallback,
        }
    }
}

/// Evaluate K and K' simultaneously, including its constant and linear terms.
#[inline]
fn eval(a: &[Cx; N + 1], w: Cx) -> (Cx, Cx) {
    let (mut v, mut d) = (Cx::default(), Cx::default());
    for x in a.iter().rev() {
        d = d.mul(w).add(v);
        v = v.mul(w).add(*x);
    }
    (v, d)
}

/// Full perturbation at C, rebasing to Z_0 at the reference period or when
/// cancellation would destroy the delta. Used when any jump guard declines.
fn fallback(m: &Mid, dc: Cx, max_iter: u64, h: f64) -> ResultPixel {
    let (mut d, mut dz, mut at) = (Cx::default(), Cx::default(), 0usize);
    for n in 1..=max_iter {
        let z = m.reference[at].add(d);
        dz = z.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
        d = m.reference[at].scale(2.0).mul(d).add(d.mul(d)).add(dc);
        at += 1;
        if at == m.reference.len() { at = 0; }
        let z = m.reference[at].add(d);
        let a = z.norm2();
        if a > R2 && a.is_finite() && dz.abs().is_finite() && dz.abs() > 0.0 {
            return ResultPixel::escaped(n, z, dz, h, true);
        }
        if a < d.norm2() {
            d = z;
            at = 0;
        }
    }
    ResultPixel::unresolved(true)
}

/// Exact perturbation of the approach, parameter-shifted cycle point and
/// first-order Koenigs series; the jump derivative includes p'(c).
fn pixel(m: &Mid, dc: Cx, max_iter: u64, h: f64) -> ResultPixel {
    if dc.abs() > 1e-6 || !dc.abs().is_finite() || max_iter <= APPROACH as u64 {
        return fallback(m, dc, max_iter, h);
    }
    let (mut d, mut dz) = (Cx::default(), Cx::default());
    for z in &m.z[..APPROACH] {
        let at = z.add(d);
        dz = at.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
        d = z.scale(2.0).mul(d).add(d.mul(d)).add(dc);
    }
    // Three implicit square-root corrections avoid subtracting c from C in f64.
    let mut ds = Cx(-4.0, 0.0).mul(dc).div(m.s.scale(2.0));
    for _ in 0..2 {
        ds = dc.scale(-4.0).div(m.s.scale(2.0).add(ds));
    }
    let dp = ds.scale(0.5 * m.sign);
    let pc = m.p.add(dp);
    let pprime = Cx(-1.0, 0.0).div(pc.scale(2.0).add(Cx(1.0, 0.0)));
    let u0 = m.u.add(d).sub(dp);
    let lam = m.lambda.add(dc.scale(4.0));
    if u0.abs() >= 0.03 / lam.abs() || !u0.abs().is_finite() {
        return fallback(m, dc, max_iter, h);
    }

    let mut k = m.k;
    for (a, b) in k.iter_mut().zip(m.kp) {
        *a = a.add(dc.mul(b));
    }
    let mut w0 = u0;
    for _ in 0..3 {
        let (v, deriv) = eval(&k, w0);
        if deriv.abs() <= 0.0 || !deriv.abs().is_finite() {
            return fallback(m, dc, max_iter, h);
        }
        w0 = w0.sub(v.sub(u0).div(deriv));
    }
    let r = w0.abs();
    if !(r > 0.0 && r.is_finite()) {
        return fallback(m, dc, max_iter, h);
    }
    let jf = (0.03 / r).ln() / lam.abs().ln();
    if !(jf.is_finite() && jf >= 1.0 && jf < m.powers.len() as f64) {
        return fallback(m, dc, max_iter, h);
    }
    let j = jf.floor() as usize;
    let n0 = APPROACH as u64 + 2 * j as u64;
    if n0 >= max_iter {
        return fallback(m, dc, max_iter, h);
    }
    // T_j = lambda(C)^j, built in high precision. The tiny parameter change
    // must be applied before landing: forming c as an f64 would lose it.
    let factor = Cx(1.0, 0.0).add(dc.scale(4.0 * j as f64).div(m.lambda));
    let lj = m.powers[j].mul(factor);
    let q = w0.mul(lj);
    if !(q.abs().is_finite() && q.abs() <= 0.031) {
        return fallback(m, dc, max_iter, h);
    }
    let (kw, dkw) = eval(&k, w0);
    let (kq, dkq) = eval(&k, q);
    let _ = kw;
    let (pcw, _) = eval(&m.kp, w0);
    let (pcq, _) = eval(&m.kp, q);
    if dkw.abs() <= 0.0 {
        return fallback(m, dc, max_iter, h);
    }
    let bracket = dz.sub(pprime).sub(pcw).add(w0.mul(dkw).mul(Cx(4.0 * j as f64, 0.0).div(lam)));
    dz = pprime.add(lj.mul(dkq).div(dkw).mul(bracket)).add(pcq);
    let mut z = pc.add(kq);
    if !z.abs().is_finite() || !dz.abs().is_finite() {
        return fallback(m, dc, max_iter, h);
    }
    let c = m.c.add(dc);
    for n in n0 + 1..=max_iter {
        dz = z.scale(2.0).mul(dz).add(Cx(1.0, 0.0));
        z = z.mul(z).add(c);
        let a = z.norm2();
        if a > R2 {
            if a.is_finite() && dz.abs().is_finite() && dz.abs() > 0.0 {
                let sample = ResultPixel::escaped(n, z, dz, h, false);
                // Close to the boundary, tiny landing errors can be amplified
                // through a long, chaotic finish. Use the independent full
                // perturbation orbit instead of trusting the jump there.
                if dc.abs() > 1e-8 && sample.de < 1.0 {
                    return fallback(m, dc, max_iter, h);
                }
                return sample;
            }
            return fallback(m, dc, max_iter, h);
        }
    }
    ResultPixel::unresolved(false)
}

/// Usage: koenigs-bench --mid CONSTS OUT_DIR NXxNY MAX_ITER THREADS RUNS WIDTH...
pub(super) fn run(args: &[String]) {
    assert!(args.len() >= 8, "usage: koenigs-bench --mid CONSTS OUT_DIR NXxNY MAX_ITER THREADS RUNS WIDTH...");
    let m = load(&args[1]);
    let out = &args[2];
    let (nx, ny) = args[3].split_once('x').map(|(x,y)| (x.parse::<usize>().unwrap(), y.parse::<usize>().unwrap())).unwrap();
    let max_iter: u64 = args[4].parse().unwrap();
    let threads: usize = args[5].parse::<usize>().unwrap().clamp(1, ny);
    let runs: usize = args[6].parse().unwrap();
    assert!(nx > 0 && ny > 0 && runs > 0);
    fs::create_dir_all(out).unwrap();
    let cols = ColumnSet::of(&[Column::Class, Column::Nu, Column::De, Column::Normal]);
    for (frame, width) in args[7..].iter().enumerate() {
        let w: f64 = width.parse().unwrap();
        assert!(w > 0.0 && w <= 1e-6);
        let h = w / nx as f64;
        let mut samples = Samples::alloc(nx * ny, cols);
        let mut times = Vec::new();
        let mut fallbacks = 0usize;
        for _ in 0..runs {
            let next = AtomicUsize::new(0);
            let t = Instant::now();
            let rows: Vec<(usize, Vec<ResultPixel>)> = std::thread::scope(|scope| {
                let workers: Vec<_> = (0..threads).map(|_| scope.spawn(|| {
                    let mut rows = Vec::new();
                    loop {
                        let y = next.fetch_add(1, Ordering::Relaxed);
                        if y >= ny { break; }
                        let im = -(y as f64 + 0.5 - ny as f64 / 2.0) * h;
                        let row = (0..nx).map(|x| {
                            let re = (x as f64 + 0.5 - nx as f64 / 2.0) * h;
                            pixel(&m, Cx(re, im), max_iter, h)
                        }).collect();
                        rows.push((y, row));
                    }
                    rows
                })).collect();
                workers.into_iter().flat_map(|w| w.join().unwrap()).collect()
            });
            times.push(t.elapsed().as_secs_f64());
            fallbacks = 0;
            for (y, row) in rows {
                for (x, p) in row.into_iter().enumerate() {
                    let i = y * nx + x;
                    samples.class[i] = Class::new(if p.escaped { Kind::Escaped } else { Kind::Unresolved }, Evidence::Heuristic);
                    samples.nu.as_mut().unwrap()[i] = p.nu;
                    samples.de.as_mut().unwrap()[i] = p.de;
                    samples.normal.as_mut().unwrap()[i] = p.normal;
                    fallbacks += usize::from(p.fallback);
                }
            }
        }
        let header = Header {
            minor: MINOR, columns: cols, nx: nx as u32, ny: ny as u32, ss: 1,
            max_iter, escape_radius: 1e10,
            view: View { center_re: m.re.clone(), center_im: m.im.clone(), width: width.clone(), rotation: 0.0 },
            kernel: "mid-koenigs-f64/1 heuristic".into(),
        };
        let path = format!("{out}/frame-{frame:05}.fds");
        write(BufWriter::new(File::create(&path).unwrap()), &header, &samples).unwrap();
        let best = times.iter().copied().fold(f64::INFINITY, f64::min);
        let escaped = samples.class.iter().filter(|c| c.escaped()).count();
        println!("{{\"mode\":\"mid\",\"width\":\"{width}\",\"nx\":{nx},\"ny\":{ny},\"threads\":{threads},\"runs\":{runs},\"best_seconds\":{best},\"all_seconds\":{times:?},\"escaped\":{escaped},\"fallbacks\":{fallbacks}}}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horner_derivative() {
        let mut k = [Cx::default(); N + 1];
        k[1] = Cx(1.0, 0.0);
        k[2] = Cx(2.0, 0.0);
        let (v, d) = eval(&k, Cx(0.3, 0.0));
        assert!((v.0 - 0.48).abs() < 1e-14);
        assert!((d.0 - 2.2).abs() < 1e-14);
    }
}
