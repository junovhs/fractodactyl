//! `fd zone`: detect a spiral-centre zone from a centre and build its fast-path constants
//! in Rust (PROB-12), replacing the mpmath generators in tools/research/misiurewicz
//! (DEC-09). This lemma (PROB-31) finds the structure: the minibrot nucleus `C` of period
//! `p`, and the Misiurewicz relation `M(q, r)` whose repelling `r`-cycle (point `A`,
//! multiplier `rho`) the critical orbit of `C` shadows before returning. It follows
//! zone_seed.py's `nucleus`/`misiurewicz` and koenigs_tail.py's cycle Newton.
use crate::args::Args;
use fd_fixed::{bits_for_digits, Complex, Float};
use std::time::Instant;

const USAGE: &str = "usage: fd zone RE IM [--max-iter N (3000)]";

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
    let a = Args::parse(argv, &["max-iter"])?;
    let [re, im] = a.positional.as_slice() else {
        return Err(USAGE.into());
    };
    let max_iter: usize = a.num("max-iter", 3000)?;
    let t = Instant::now();
    let s = seed(re, im, max_iter)?;
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
