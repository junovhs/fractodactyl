//! The f32-only zone kernel's float-float library (GPU-27, shaders/ff.wgsl), checked
//! on the GPU against binary64. Production code includes ff.wgsl into the zone kernel
//! (GPU-28); this module only tests it.

/// The library (f32 and integer arithmetic only, DEC-25). The f32-only zone kernel
/// (GPU-28) is its first production user.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const FF: &str = include_str!("../shaders/ff.wgsl");

#[cfg(test)]
mod tests {
    use fd_shade::gpu::testkit::{dispatch, or_skip, Binding};

    /// One op per case: [op, payload x6, pad] in, two vec4<u32> out.
    const ENTRY: &str = r#"
@group(0) @binding(0) var<storage, read> cases: array<u32>;
@group(0) @binding(1) var<storage, read> key: array<u32>;
@group(0) @binding(2) var<storage, read_write> out: array<u32>;

fn arg(i: u32, k: u32) -> FF {
    return FF(bitcast<f32>(cases[8u * i + 1u + 2u * k]), bitcast<f32>(cases[8u * i + 2u + 2u * k]));
}
fn put(i: u32, x: FF, e: i32, w: FF) {
    out[8u * i] = bitcast<u32>(x.hi);
    out[8u * i + 1u] = bitcast<u32>(x.lo);
    out[8u * i + 2u] = bitcast<u32>(e);
    out[8u * i + 3u] = select(0u, 1u, FF_FUSED);
    out[8u * i + 4u] = bitcast<u32>(w.hi);
    out[8u * i + 5u] = bitcast<u32>(w.lo);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x + id.y * 65535u * 64u;
    if (i * 8u >= arrayLength(&cases)) {
        return;
    }
    ff_init(key[0]);
    let op = cases[8u * i];
    let a = arg(i, 0u);
    let b = arg(i, 1u);
    let v = fe_from_f64(cases[8u * i + 1u], cases[8u * i + 2u]);
    let w = fe_from_f64(cases[8u * i + 3u], cases[8u * i + 4u]);
    switch op {
        case 0u: { put(i, ff_add(a, b), 0, ff(0.0)); }
        case 1u: { put(i, ff_mul(a, b), 0, ff(0.0)); }
        case 2u: { put(i, ff_div(a, b), 0, ff(0.0)); }
        case 3u: { put(i, ff_sqrt(a), 0, ff(0.0)); }
        case 4u: { put(i, ff_exp(a), 0, ff(0.0)); }
        case 5u: { put(i, ff_log(a), 0, ff(0.0)); }
        case 6u: { let sc = ff_sincos(a); put(i, sc.s, 0, sc.c); }
        case 7u: { put(i, ff_atan2(a, b), 0, ff(0.0)); }
        case 8u: { put(i, v.m, v.e, ff(0.0)); }
        case 9u: { let p = fe_mul(v, w); put(i, p.m, p.e, ff(0.0)); }
        case 10u: { let s = fe_add(v, w); put(i, s.m, s.e, ff(0.0)); }
        default: {
            // (v + i w)^2 as a CE.
            let z = ce_add(CE(v.m, ff(0.0), v.e), CE(ff(0.0), w.m, w.e));
            let q = ce_mul(z, z);
            put(i, q.re, q.e, q.im);
        }
    }
}
"#;

    /// xorshift64*.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }
        /// Uniform in [lo, hi).
        fn range(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
        /// Sign * 2^[lo, hi) * [1, 2).
        fn wide(&mut self, lo: i32, hi: i32) -> f64 {
            let e = lo + (self.next() % (hi - lo) as u64) as i32;
            let s = if self.next() & 1 == 0 { 1.0 } else { -1.0 };
            s * self.range(1.0, 2.0) * 2f64.powi(e)
        }
    }

    /// x as the nearest float-float, and the value it holds exactly.
    fn split(x: f64) -> ([f32; 2], f64) {
        let hi = x as f32;
        let lo = (x - f64::from(hi)) as f32;
        ([hi, lo], f64::from(hi) + f64::from(lo))
    }

    struct Case {
        op: u32,
        words: [u32; 6],
        /// The reference result(s) in binary64 and the allowed error of each.
        want: [f64; 2],
        tol: [f64; 2],
        what: &'static str,
    }

    fn ff_case(op: u32, a: f64, b: f64, f: impl Fn(f64, f64) -> ([f64; 2], [f64; 2])) -> Case {
        let ((sa, xa), (sb, xb)) = (split(a), split(b));
        let (want, tol) = f(xa, xb);
        Case {
            op,
            words: [
                sa[0].to_bits(),
                sa[1].to_bits(),
                sb[0].to_bits(),
                sb[1].to_bits(),
                0,
                0,
            ],
            want,
            tol,
            what: ["add", "mul", "div", "sqrt", "exp", "log", "sincos", "atan2"][op as usize],
        }
    }

    fn fe_case(op: u32, v: f64, w: f64, what: &'static str, want: [f64; 2], tol: f64) -> Case {
        let (v, w) = (v.to_bits(), w.to_bits());
        Case {
            op,
            words: [v as u32, (v >> 32) as u32, w as u32, (w >> 32) as u32, 0, 0],
            want,
            tol: [tol; 2],
            what,
        }
    }

    const REL: f64 = 1.0 / (1u64 << 44) as f64;
    const TRANS: f64 = 1.0 / (1u64 << 40) as f64;

    fn cases() -> Vec<Case> {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut v = Vec::new();
        for _ in 0..4000 {
            // Wide magnitudes, near cancellation, ties at the f32 boundary.
            // FF alone covers |x| in 2^[-100, 100]: below that its low word is subnormal
            // and GPUs flush it (the FE type covers the rest).
            let a = rng.wide(-45, 45);
            let near = a * (1.0 + rng.range(-1e-6, 1e-6));
            for b in [rng.wide(-45, 45), -near, -a * (1.0 + rng.wide(-40, -24))] {
                v.push(ff_case(0, a, b, |x, y| {
                    ([x + y, 0.0], [REL * (x.abs() + y.abs()), 0.0])
                }));
            }
            let b = rng.wide(-45, 45);
            v.push(ff_case(1, a, b, |x, y| {
                ([x * y, 0.0], [REL * (x * y).abs(), 0.0])
            }));
            v.push(ff_case(2, a, b, |x, y| {
                ([x / y, 0.0], [REL * (x / y).abs(), 0.0])
            }));
            let p = a.abs();
            v.push(ff_case(3, p, 0.0, |x, _| {
                ([x.sqrt(), 0.0], [REL * x.sqrt(), 0.0])
            }));
            let e = rng.range(-60.0, 60.0);
            v.push(ff_case(4, e, 0.0, |x, _| {
                ([x.exp(), 0.0], [TRANS * x.exp(), 0.0])
            }));
            let l = rng.wide(-60, 60).abs();
            let one = 1.0 + rng.range(-1e-3, 1e-3);
            for x in [l, one] {
                v.push(ff_case(5, x, 0.0, |x, _| {
                    ([x.ln(), 0.0], [TRANS * x.ln().abs().max(1.0), 0.0])
                }));
            }
            let t = rng.range(-1000.0, 1000.0);
            let edge = std::f64::consts::FRAC_PI_4 * f64::from(rng.next() as u32 % 64)
                - 8.0 * std::f64::consts::PI;
            for x in [t, edge, rng.range(-1e-6, 1e-6)] {
                v.push(ff_case(6, x, 0.0, |x, _| ([x.sin(), x.cos()], [TRANS; 2])));
            }
            let (y, x) = (rng.wide(-40, 40), rng.wide(-40, 40));
            v.push(ff_case(7, y, x, |y, x| ([y.atan2(x), 0.0], [TRANS, 0.0])));
            // FE: f64 magnitudes far outside f32's range.
            let (fv, fw) = (rng.wide(-1000, 1000), rng.wide(-1000, 1000));
            v.push(fe_case(
                8,
                fv,
                0.0,
                "fe_from_f64",
                [fv, 0.0],
                REL * fv.abs(),
            ));
            if (fv * fw).is_normal() {
                v.push(fe_case(
                    9,
                    fv,
                    fw,
                    "fe_mul",
                    [fv * fw, 0.0],
                    REL * (fv * fw).abs(),
                ));
            }
            let fw2 = fv * rng.range(-3.0, 3.0) * 2f64.powi(rng.next() as i32 % 40);
            // fe_from_f64 maps f64 subnormals to zero (documented); keep inputs normal.
            if fw2.is_normal() && (fv + fw2).is_normal() {
                let sum_tol = REL * (fv.abs() + fw2.abs());
                v.push(fe_case(10, fv, fw2, "fe_add", [fv + fw2, 0.0], sum_tol));
            }
            let (cv, cw) = (rng.wide(-400, 400), rng.wide(-400, 400));
            let sq = [cv * cv - cw * cw, 2.0 * cv * cw];
            v.push(fe_case(
                11,
                cv,
                cw,
                "ce_mul",
                sq,
                4.0 * REL * (cv * cv + cw * cw),
            ));
        }
        v
    }

    /// Runs every case with the guard on or off; returns the adapter, whether fma was
    /// fused, and the failures as text.
    fn run(guard: bool) -> Option<(String, bool, Vec<String>, usize)> {
        let cases = cases();
        let words: Vec<u8> = cases
            .iter()
            .flat_map(|c| {
                std::iter::once(c.op)
                    .chain(c.words)
                    .chain([0])
                    .flat_map(u32::to_ne_bytes)
            })
            .collect();
        let source = format!("{}\n{ENTRY}", super::FF);
        let source = if guard {
            source
        } else {
            source.replace(
                "override FF_GUARD: bool = true;",
                "override FF_GUARD: bool = false;",
            )
        };
        let blocks = (cases.len() as u32).div_ceil(64);
        let (name, out) = or_skip(
            dispatch(
                "fd zone ff test",
                &source,
                "main",
                &[
                    Binding::Read(&words),
                    Binding::Read(&0u32.to_ne_bytes()),
                    Binding::Out(cases.len() as u64 * 32),
                ],
                (blocks.min(65535), blocks.div_ceil(65535)),
            ),
            "zone ff",
        )?;
        let w: Vec<u32> = out
            .chunks_exact(4)
            .map(|b| u32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        let ff = |hi: u32, lo: u32| f64::from(f32::from_bits(hi)) + f64::from(f32::from_bits(lo));
        // m 2^e in two steps, so 2^e itself may exceed f64's range.
        let scaled = |m: f64, e: i32| m * 2f64.powi(e / 2) * 2f64.powi(e - e / 2);
        let mut fails = Vec::new();
        for (i, c) in cases.iter().enumerate() {
            let o = &w[8 * i..8 * i + 8];
            let e = o[2] as i32;
            let got = match c.op {
                8..=10 => [scaled(ff(o[0], o[1]), e), 0.0],
                11 => [scaled(ff(o[0], o[1]), e), scaled(ff(o[4], o[5]), e)],
                _ => [ff(o[0], o[1]), ff(o[4], o[5])],
            };
            let n = if c.op == 6 || c.op == 11 { 2 } else { 1 };
            for (k, (&g, (&want, &tol))) in got
                .iter()
                .zip(c.want.iter().zip(&c.tol))
                .take(n)
                .enumerate()
            {
                // NaN fails too: only a finite error within the bound passes.
                let err = (g - want).abs();
                if err.is_nan() || err > tol {
                    fails.push(format!(
                        "{} case {i} part {k}: got {g:e}, want {want:e}, err {err:e} > {tol:e}",
                        c.what
                    ));
                }
            }
        }
        Some((name, w[3] == 1, fails, cases.len()))
    }

    #[test]
    fn zone_ff_helpers_match_binary64() {
        let Some((name, fused, fails, n)) = run(true) else {
            return;
        };
        eprintln!(
            "adapter {name}: TwoProd by {}; {n} cases, {} failures",
            if fused {
                "fma (fused)"
            } else {
                "Dekker (fma split)"
            },
            fails.len()
        );
        for f in fails.iter().take(20) {
            eprintln!("  {f}");
        }
        assert!(fails.is_empty(), "{} ff cases out of bounds", fails.len());
    }

    /// The red check: with the reassociation guard stripped, compilers fold the
    /// error terms away (Mesa NIR rewrites a + b - a to b) and the helpers fail.
    #[test]
    fn zone_ff_guard_is_needed() {
        let Some((name, _, fails, n)) = run(false) else {
            return;
        };
        eprintln!(
            "adapter {name}, guard stripped: {} of {n} cases fail",
            fails.len()
        );
        // DXC (DX12 WARP) keeps the error terms even unguarded; Mesa's NIR (RADV,
        // lavapipe) folds them, and NVIDIA and Intel drivers did on GPU-24.
        if name.contains("Microsoft Basic Render Driver") {
            return;
        }
        assert!(!fails.is_empty(), "the guard did nothing on {name}");
    }
}
