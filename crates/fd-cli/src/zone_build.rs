//! `fd zone`: detect a spiral-centre zone from a centre and build its fast-path constants
//! in Rust (PROB-12), replacing the mpmath generators in tools/research/misiurewicz
//! (DEC-09). This lemma (PROB-31) finds the structure: the minibrot nucleus `C` of period
//! `p`, and the Misiurewicz relation `M(q, r)` whose repelling `r`-cycle (point `A`,
//! multiplier `rho`) the critical orbit of `C` shadows before returning. It follows
//! zone_seed.py's `nucleus`/`misiurewicz` and koenigs_tail.py's cycle Newton. PROB-32
//! adds the first-return biseries and its diagonal series (returns_exit_tail.py
//! `build_map_mp`, koenigs_bench_consts.py `diagonal`); PROB-33 the Koenigs chart phi at
//! A and psi = phi^-1 (koenigs_tail.py `setup`, psi_series.py `reverse`).
use crate::args::Args;
use fd_fixed::{bits_for_digits, Complex, Float};
use std::time::Instant;

const USAGE: &str = "usage: fd zone RE IM [--max-iter N (3000)] [--emit report|lines]";

/// Biseries degree (make_zone.sh: koenigs_bench_consts.py DEGREE 4).
const DEGREE: usize = 4;
/// Koenigs series terms written (make_zone.sh: TERMS 12 and PSI_TERMS 18).
const PHI_TERMS: usize = 12;
const PSI_TERMS: usize = 18;

/// Longest cycle tried for the Misiurewicz relation (zone_seed.py `RMAX`).
const RMAX: usize = 8;

/// A zone's structure: nucleus, period and the Misiurewicz cycle it shadows.
pub(crate) struct Seed {
    pub(crate) period: usize,
    /// The nucleus, Newton-refined at `digits`.
    pub(crate) c: Complex,
    pub(crate) q: usize,
    pub(crate) r: usize,
    pub(crate) alpha: Complex,
    pub(crate) rho: Complex,
    /// Laps of the r-cycle the nucleus orbit shadows from step q (|z_{n+r} - z_n| < 1e-2).
    pub(crate) laps: usize,
    pub(crate) digits: usize,
}

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["max-iter", "emit"])?;
    let [re, im] = a.positional.as_slice() else {
        return Err(USAGE.into());
    };
    let max_iter: usize = a.num("max-iter", 3000)?;
    let t = Instant::now();
    let s = seed(re, im, max_iter)?;
    match a.str("emit").unwrap_or("report") {
        "report" => {}
        "lines" => {
            let k = koenigs(&s, PSI_TERMS);
            print!("{}", lines(&s, &return_map(&s, DEGREE), &k));
            return Ok(());
        }
        e => return Err(format!("--emit: expected report or lines, got {e:?}")),
    }
    let pair = |z: &Complex| format!("[{:?},{:?}]", z.re.to_f64(), z.im.to_f64());
    println!(
        "{{\"schema\":\"fd-zone/1\",\"period\":{},\"c_exact\":[\"{}\",\"{}\"],\"q\":{},\"r\":{},\"alpha\":{},\"rho\":{},\"rho_abs\":{:?},\"laps\":{},\"digits\":{},\"seconds\":{:.3}}}",
        s.period,
        s.c.re.to_decimal(s.digits - 2),
        s.c.im.to_decimal(s.digits - 2),
        s.q,
        s.r,
        pair(&s.alpha),
        pair(&s.rho),
        (log2_norm2(&s.rho) / 2.0).exp2(),
        s.laps,
        s.digits,
        t.elapsed().as_secs_f64()
    );
    Ok(())
}

/// `log2 |x|` of a real at any depth (`-inf` for zero).
fn log2_abs(x: &Float) -> f64 {
    let (m, e) = x.frexp();
    if m == 0.0 {
        f64::NEG_INFINITY
    } else {
        m.abs().log2() + e as f64
    }
}

/// `log2 |z|^2`.
fn log2_norm2(z: &Complex) -> f64 {
    log2_abs(&z.norm2())
}

/// The critical orbit z_0 = 0, z_1 = c, ... and u_n = dz_n/dc, `n` steps or until
/// |z| > 1e6.
fn orbit(c: &Complex, n: usize, bits: usize) -> (Vec<Complex>, Vec<Complex>) {
    let one = Complex::from_i64(1, bits);
    let (mut z, mut u) = (vec![Complex::zero(bits)], vec![Complex::zero(bits)]);
    for _ in 0..n {
        let (zl, ul) = (z.last().unwrap(), u.last().unwrap());
        u.push(zl.mul(ul).ldexp(1).add(&one));
        z.push(zl.sqr().add(c));
        if log2_norm2(z.last().unwrap()) > 2.0 * 6.0 * std::f64::consts::LOG2_10 {
            break;
        }
    }
    (z, u)
}

/// Working digits for period `p` (koenigs_bench_consts.py): the reference orbit loses
/// roughly one digit per 16 steps beyond v0's 764.
fn digits_for(p: usize) -> usize {
    let depth = 50 + p.saturating_sub(764) / 16;
    (2 * depth + 70).max(80)
}

pub(crate) fn seed(re: &str, im: &str, max_iter: usize) -> Result<Seed, String> {
    // The record return at the given centre's own precision: the n with the smallest |z_n|.
    let given = re.len().max(im.len()).max(40) + 20;
    let c0 = Complex::parse(re, im, bits_for_digits(given))?;
    let (z, _) = orbit(&c0, max_iter, bits_for_digits(given));
    let p = (2..z.len())
        .min_by(|&a, &b| log2_norm2(&z[a]).total_cmp(&log2_norm2(&z[b])))
        .ok_or("the orbit escapes before any return")?;
    let digits = digits_for(p);
    let bits = bits_for_digits(digits);
    let c = nucleus(&Complex::parse(re, im, bits)?, p, digits, bits)?;
    let (q, r) = misiurewicz(&c, p, bits)?;
    let (z, _) = orbit(&c, p + RMAX + 2, bits);
    let (alpha, rho) = cycle(&c, &z[q], r, digits, bits)?;
    if log2_norm2(&rho) <= 0.0 {
        return Err(format!(
            "the M({q},{r}) cycle is not repelling (|rho| = {:.4})",
            (log2_norm2(&rho) / 2.0).exp2()
        ));
    }
    let close = |n: usize| z[n + r].sub(&z[n]).norm2().to_f64() < 1e-4;
    let mut n = q;
    while n + r < p && close(n) {
        n += 1;
    }
    let laps = (n - q) / r;
    if laps < 2 {
        return Err(format!(
            "the orbit shadows the M({q},{r}) cycle for {laps} laps; a Koenigs jump needs 2"
        ));
    }
    Ok(Seed {
        period: p,
        c,
        q,
        r,
        alpha,
        rho,
        laps,
        digits,
    })
}

/// Newton on the period-`p` nucleus equation z_p(c) = 0 from `c`.
fn nucleus(c: &Complex, p: usize, digits: usize, bits: usize) -> Result<Complex, String> {
    let one = Complex::from_i64(1, bits);
    let tol = -2.0 * (digits as f64 - 25.0) * std::f64::consts::LOG2_10;
    let mut c = c.clone();
    for _ in 0..80 {
        let (mut z, mut dz) = (Complex::zero(bits), Complex::zero(bits));
        for _ in 0..p {
            dz = z.mul(&dz).ldexp(1).add(&one);
            z = z.sqr().add(&c);
        }
        let step = z.div(&dz);
        c = c.sub(&step);
        if log2_norm2(&step) < tol {
            return Ok(c);
        }
    }
    Err(format!("the period-{p} nucleus Newton did not converge"))
}

/// The relation `(q, r)` minimising D = |(z_{q+r} - z_q) / (u_{q+r} - u_q)| over
/// r <= RMAX and q < p/2, reduced to the smallest preperiod with the same root.
fn misiurewicz(c: &Complex, p: usize, bits: usize) -> Result<(usize, usize), String> {
    let (z, u) = orbit(c, p + RMAX + 2, bits);
    // log2 D, or None where u_{q+r} = u_q.
    let log_d = |q: usize, r: usize| {
        let dh = u[q + r].sub(&u[q]);
        (!dh.is_zero()).then(|| (log2_norm2(&z[q + r].sub(&z[q])) - log2_norm2(&dh)) / 2.0)
    };
    let mut best = (f64::INFINITY, 0, 0);
    for r in 1..=RMAX {
        for q in 1..p / 2 {
            if let Some(d) = log_d(q, r) {
                // `D < best * 0.999`: a near tie keeps the shorter cycle.
                if d < best.0 + 0.999f64.log2() {
                    best = (d, q, r);
                }
            }
        }
    }
    let (d, mut q, r) = best;
    if r == 0 {
        return Err("no Misiurewicz relation found".into());
    }
    while q > 1 {
        match log_d(q - 1, r) {
            Some(e) if ((e - d).exp2() - 1.0).abs() <= 1e-3 => q -= 1,
            _ => break,
        }
    }
    Ok((q, r))
}

/// Newton for the r-cycle point A near `start` (f^r(A) = A), and its multiplier
/// rho = prod 2 a over the cycle.
fn cycle(
    c: &Complex,
    start: &Complex,
    r: usize,
    digits: usize,
    bits: usize,
) -> Result<(Complex, Complex), String> {
    let one = Complex::from_i64(1, bits);
    // f^r(a) and its derivative in z.
    let map = |a: &Complex| {
        let (mut w, mut d) = (a.clone(), one.clone());
        for _ in 0..r {
            d = d.mul(&w).ldexp(1);
            w = w.sqr().add(c);
        }
        (w, d)
    };
    let tol = -2.0 * (digits as f64 - 5.0) * std::f64::consts::LOG2_10;
    let mut a = start.clone();
    for _ in 0..80 {
        let (w, d) = map(&a);
        let step = w.sub(&a).div(&d.sub(&one));
        a = a.sub(&step);
        if log2_norm2(&step) < tol {
            let (_, rho) = map(&a);
            return Ok((a, rho));
        }
    }
    Err(format!("the {r}-cycle Newton did not converge"))
}

/// The first return to the nucleus over one period, as a degree-`DEGREE` biseries in
/// u = (z - C)/scale and v = (c - C)/scale (PROB-07): u' = bias + sum a_ij u^i v^j.
pub(crate) struct ReturnMap {
    pub(crate) scale: Float,
    /// (i, j, a_ij) for 0 < i + j <= DEGREE, i outer.
    pub(crate) coeff: Vec<(usize, usize, Complex)>,
    pub(crate) bias: Complex,
    /// S_1..S_{DEGREE + 2} of the return from u = v, M(v, v) = sum S_n v^n (PROB-20).
    pub(crate) diag: Vec<Complex>,
}

/// The zone scale, 10^floor(log10(size) / 2) for the minibrot size 1/|b dz/dc|
/// (koenigs_bench_consts.py): return-map states scale as the square root of the size.
fn scale(s: &Seed, bits: usize) -> Float {
    let one = Complex::from_i64(1, bits);
    let (mut z, mut dz) = (Complex::zero(bits), Complex::zero(bits));
    let mut log2_b = 0.0;
    for j in 0..s.period {
        dz = z.mul(&dz).ldexp(1).add(&one);
        z = z.sqr().add(&s.c);
        if j + 1 < s.period {
            log2_b += 1.0 + log2_norm2(&z) / 2.0;
        }
    }
    let log10_size = -(log2_b + log2_norm2(&dz) / 2.0) / std::f64::consts::LOG2_10;
    let k = (log10_size / 2.0).floor() as i64;
    Float::parse(&format!("1e{k}"), bits).expect("power of ten parses")
}

pub(crate) fn return_map(s: &Seed, degree: usize) -> ReturnMap {
    let bits = s.c.re.precision();
    let scale = scale(s, bits);
    let terms: Vec<(usize, usize)> = (0..=degree)
        .flat_map(|i| (0..=degree - i).map(move |j| (i, j)))
        .filter(|&(i, j)| i + j > 0)
        .collect();
    let zero = Complex::zero(bits);
    let one = Complex::from_i64(1, bits);
    // z = C + scale t, c = C + scale v: t' = 2 Z t + scale t^2 + v, truncated above
    // total degree `degree` with every mixed term kept.
    let mut a = vec![vec![zero.clone(); degree + 1]; degree + 1];
    a[1][0] = one.clone();
    let mut z = s.c.clone();
    for _ in 0..s.period {
        let two_z = z.ldexp(1);
        let mut b: Vec<Vec<Complex>> = a
            .iter()
            .map(|row| row.iter().map(|x| two_z.mul(x)).collect())
            .collect();
        for &(i, j) in &terms {
            if a[i][j].is_zero() {
                continue;
            }
            let sx = a[i][j].scale(&scale);
            for &(h, l) in &terms {
                if i + j + h + l <= degree && !a[h][l].is_zero() {
                    b[i + h][j + l] = b[i + h][j + l].add(&sx.mul(&a[h][l]));
                }
            }
        }
        b[0][1] = b[0][1].add(&one);
        a = b;
        z = z.sqr().add(&s.c);
    }
    let bias = z.sub(&s.c).scale(&Float::from_i64(1, bits).div(&scale));
    let coeff = terms
        .iter()
        .map(|&(i, j)| (i, j, a[i][j].clone()))
        .collect();
    ReturnMap {
        diag: diagonal(s, &scale, degree + 2),
        scale,
        coeff,
        bias,
    }
}

/// The first return from u = v as one series in v: S_1..S_n (the biseries' diagonal
/// sums to its degree, plus the terms its truncation drops).
fn diagonal(s: &Seed, scale: &Float, n: usize) -> Vec<Complex> {
    let bits = s.c.re.precision();
    let one = Complex::from_i64(1, bits);
    let mut a = vec![Complex::zero(bits); n + 1];
    a[1] = one.clone();
    let mut z = s.c.clone();
    for _ in 0..s.period {
        let two_z = z.ldexp(1);
        let mut b: Vec<Complex> = a.iter().map(|x| two_z.mul(x)).collect();
        for i in 1..n {
            let sx = a[i].scale(scale);
            for j in 1..=n - i {
                b[i + j] = b[i + j].add(&sx.mul(&a[j]));
            }
        }
        b[1] = b[1].add(&one);
        a = b;
        z = z.sqr().add(&s.c);
    }
    a.split_off(1)
}

/// `z` as a mantissa pair with a shared binary exponent, the larger part in [1, 2):
/// the `_x` and `diag` line format, exact where the doubles leave f64 (DEC-21).
fn fx(z: &Complex) -> (f64, f64, i64) {
    let big = if z.re.abs().lt(&z.im.abs()) {
        &z.im
    } else {
        &z.re
    };
    if big.is_zero() {
        return (0.0, 0.0, 0);
    }
    let e = big.frexp().1 - 1;
    (z.re.ldexp(-e).to_f64(), z.im.ldexp(-e).to_f64(), e)
}

/// Zone-file lines (KERN-01's format) for the constants built so far.
fn lines(s: &Seed, m: &ReturnMap, k: &Koenigs) -> String {
    use std::fmt::Write as _;
    let c = |z: &Complex| format!("{:?} {:?}", z.re.to_f64(), z.im.to_f64());
    let x = |z: &Complex| {
        let (re, im, e) = fx(z);
        format!("{re:?} {im:?} {e}")
    };
    let (sm, se) = m.scale.frexp();
    let mut o = String::new();
    let _ = writeln!(o, "# fd zone (PROB-12): degree {DEGREE}");
    let _ = writeln!(
        o,
        "c_exact {} {}",
        s.c.re.to_decimal(s.digits - 2),
        s.c.im.to_decimal(s.digits - 2)
    );
    let _ = writeln!(o, "c {}", c(&s.c));
    let _ = writeln!(o, "period {}", s.period);
    let _ = writeln!(o, "scale {:?} {}", 2.0 * sm, se - 1);
    let _ = writeln!(o, "bias {}", c(&m.bias));
    let _ = writeln!(o, "alpha {}", c(&s.alpha));
    let _ = writeln!(o, "rho {}", c(&s.rho));
    for (i, j, a) in &m.coeff {
        let _ = writeln!(o, "biseries {i} {j} {}", c(a));
    }
    for (i, j, a) in &m.coeff {
        let _ = writeln!(o, "biseries_x {i} {j} {}", x(a));
    }
    for (n, d) in m.diag.iter().enumerate() {
        let _ = writeln!(o, "diag {} {}", n + 1, x(d));
    }
    for (n, a) in k.phi.iter().take(PHI_TERMS).enumerate() {
        let _ = writeln!(o, "phi {} {}", n + 1, c(a));
    }
    for (n, a) in k.psi.iter().enumerate() {
        let _ = writeln!(o, "psi {} {}", n + 1, c(a));
    }
    o
}

/// The Koenigs chart at the r-cycle point A: phi linearises g(h) = f^r(A + h) - A,
/// phi(g(h)) = rho phi(h) with phi'(0) = 1, and psi = phi^-1 (PROB-09). `phi[k - 1]` and
/// `psi[k - 1]` are the coefficients of h^k and w^k.
pub(crate) struct Koenigs {
    pub(crate) phi: Vec<Complex>,
    pub(crate) psi: Vec<Complex>,
}

/// Power series product truncated after degree `n` (index = power).
fn series_mul(x: &[Complex], y: &[Complex], n: usize) -> Vec<Complex> {
    let bits = x[0].re.precision();
    let mut out = vec![Complex::zero(bits); n + 1];
    for (i, xi) in x.iter().enumerate().take(n + 1) {
        if xi.is_zero() {
            continue;
        }
        for (j, yj) in y.iter().enumerate().take(n + 1 - i) {
            if !yj.is_zero() {
                out[i + j] = out[i + j].add(&xi.mul(yj));
            }
        }
    }
    out
}

/// phi and psi to `n` terms. A coefficient of index k depends only on lower ones, so the
/// truncation at `n` leaves every written term exact.
pub(crate) fn koenigs(s: &Seed, n: usize) -> Koenigs {
    let bits = s.c.re.precision();
    let zero = Complex::zero(bits);
    // g(h) = f^r(A + h) - A by composing f r times on the series A + h.
    let mut p = vec![zero.clone(); n + 1];
    p[0] = s.alpha.clone();
    p[1] = Complex::from_i64(1, bits);
    for _ in 0..s.r {
        p = series_mul(&p, &p, n);
        p[0] = p[0].add(&s.c);
    }
    let mut g = p;
    g[0] = zero.clone();
    let rho = g[1].clone();
    let mut powers = vec![Vec::new(), g.clone()];
    for k in 2..=n {
        let next = series_mul(&powers[k - 1], &g, n);
        powers.push(next);
    }
    // phi(g(h)) = rho phi(h), term by term: phi_n (rho^n - rho) = -sum_{k<n} phi_k [h^n] g^k.
    let mut phi = vec![zero.clone(); n + 1];
    phi[1] = Complex::from_i64(1, bits);
    let mut rho_n = rho.clone();
    for m in 2..=n {
        rho_n = rho_n.mul(&rho);
        let mut acc = zero.clone();
        for k in 1..m {
            acc = acc.add(&phi[k].mul(&powers[k][m]));
        }
        phi[m] = acc.neg().div(&rho_n.sub(&rho));
    }
    // psi by series reversion: the w^m term of sum_k phi_k psi^k vanishes for m > 1.
    let mut psi = vec![zero.clone(); n + 1];
    psi[1] = Complex::from_i64(1, bits);
    for m in 2..=n {
        let mut q = psi.clone();
        q[m] = zero.clone();
        let mut pw = q.clone();
        let mut acc = zero.clone();
        for (k, phi_k) in phi.iter().enumerate().take(m + 1).skip(1) {
            if k > 1 {
                pw = series_mul(&pw, &q, n);
            }
            acc = acc.add(&phi_k.mul(&pw[m]));
        }
        psi[m] = acc.neg();
    }
    phi.remove(0);
    psi.remove(0);
    Koenigs { phi, psi }
}
