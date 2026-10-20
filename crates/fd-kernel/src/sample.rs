//! One sample: perturbed iteration with rebasing (Zhuoran), Brent periodicity check
//! confirmed by Newton, and the derivative only when a column needs it. With `B`, also a
//! rigorous running error radius on `z` and `dz/dc` (Bounded evidence, docs/spec/LOD.md).
//! With a BLA table, valid blocks replace runs of steps (ACC-01, not with `B`).
use crate::bla::{Bla, MIN_PROFITABLE_STEPS};
use crate::interior::{attracting, contracting, in_main_components};
pub(crate) use crate::number::Binary64Exponent;
use crate::number::{Arithmetic, Binary64};
use crate::reference::Reference;
use fd_fixed::exp2i;

const HI: f64 = 4294967296.0;
const LO: f64 = 1.0 / HI;
const SQUARE_OK: f64 = 1e-290;
const TINY: f64 = 1.0 / 1e270;

/// Result of one sample, before quantisation into columns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Outcome {
    /// `z` and `dz/dc = (dr, di) * 2^dexp` at the first iterate outside the escape
    /// radius (`n` iterates), with error radii `ez >= |z - z_exact|` and
    /// `ed >= |(dr, di) - dz/dc_exact * 2^-dexp|` (infinite when not tracked).
    Escaped {
        n: u64,
        zr: f64,
        zi: f64,
        dr: f64,
        di: f64,
        dexp: i64,
        ez: f64,
        ed: f64,
    },
    /// Judged bounded after `n` iterates (0 for the closed-form main-component test).
    Interior {
        n: u64,
    },
    Unresolved,
}

impl Outcome {
    /// Perturbation iterates this sample spent (`max_iter` when unresolved).
    #[inline]
    pub(crate) fn iterations(self, max_iter: u64) -> u64 {
        match self {
            Outcome::Escaped { n, .. } | Outcome::Interior { n } => n,
            Outcome::Unresolved => max_iter,
        }
    }
}

/// Unit roundoff of f64.
pub(crate) const U: f64 = f64::EPSILON / 2.0;
/// Covers the rounding of the error-radius arithmetic itself.
const SAFE: f64 = 1.0 + 8.0 * U;

/// BLA work of one sample: blocks applied, the steps they replaced, and (with `D`) the
/// first-order shift estimate: the sum over applied blocks of the remainder bound
/// `alpha |delta_m| + beta |dc|` divided by `|dz/dc|` at the landing point, the distance
/// in `c` that moves the sample as much as the dropped terms do (ATLAS.md "BLA tables").
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Skip {
    pub(crate) blocks: u64,
    pub(crate) skipped: u64,
    pub(crate) shift: f64,
}

/// [`perturb`] without BLA.
#[cfg(test)]
pub(crate) fn sample<const D: bool, const B: bool>(
    r: &Reference,
    q: &[f64],
    c: Option<(f64, f64)>,
    ar: f64,
    ai: f64,
    max_iter: u64,
    r2: f64,
) -> Outcome {
    perturb::<D, B, false>(
        r,
        q,
        None,
        &mut Skip::default(),
        c,
        ar,
        ai,
        max_iter,
        r2,
        r.nucleus_period(),
    )
}

/// Iterate sample `C + (ar, ai)`. `D` selects whether `dz/dc` is tracked. `c` is the
/// absolute sample position when f64 resolves it, enabling the closed-form interior test.
/// `B` (needs `D`) tracks error radii against the exact orbit, using `q` from
/// [`Reference::error_radius`]; `q` is unused otherwise. With `L` and `bla` (built over
/// `r` for `|(ar, ai)| <= dc_max`; never with `B`), valid blocks replace runs of steps,
/// counted in `skip`. `L` is a constant so the plain kernel carries no BLA test.
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn perturb<const D: bool, const B: bool, const L: bool>(
    r: &Reference,
    q: &[f64],
    bla: Option<&Bla>,
    skip: &mut Skip,
    c: Option<(f64, f64)>,
    ar: f64,
    ai: f64,
    max_iter: u64,
    r2: f64,
    nucleus: Option<usize>,
) -> Outcome {
    iterate::<Binary64, D, B, L>(r, q, bla, skip, c, ar, ai, 0, max_iter, r2, nucleus)
}

// DEC-24: one perturbation loop, instantiated for each arithmetic representation.
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) fn iterate<A: Arithmetic, const D: bool, const B: bool, const L: bool>(
    r: &Reference,
    q: &[f64],
    bla: Option<&Bla>,
    skip: &mut Skip,
    c: Option<(f64, f64)>,
    ar: f64,
    ai: f64,
    offset_exp: i64,
    max_iter: u64,
    r2: f64,
    nucleus: Option<usize>,
) -> Outcome {
    debug_assert!(!A::EXTENDED || (!B && !L && c.is_none()));
    if c.is_some_and(|c| in_main_components(c.0, c.1)) {
        return Outcome::Interior { n: 0 };
    }
    let (zr, zi, last) = (&r.re, &r.im, r.len() - 1);
    let (mut xr, mut xi) = (0.0f64, 0.0f64); // delta z / 2^delta_exp
    let mut delta_exp = if A::EXTENDED { offset_exp } else { 0 };
    let mut scale = if A::EXTENDED { exp2i(delta_exp) } else { 1.0 };
    let (mut dc_r, mut dc_i) = (ar, ai);
    let (mut dr, mut di) = (0.0f64, 0.0f64); // dz/dc
    let (mut derivative_exp, mut inverse) = (0i64, 1.0f64);
    let (mut m, mut n) = (0usize, 0u64);
    // Brent: compare against the point saved (at iterate `sn`) at the last power of two.
    let (mut sr, mut si, mut chk, mut sn) =
        (0.0f64, 0.0f64, 16u64, if A::EXTENDED { 0u64 } else { 8u64 });
    let (mut newton_at, mut tries, mut returns) = (64u64, 0, 0);
    // Error radii (B): e >= |z_exact - (A_m + delta)| with A the exact reference orbit,
    // ed on dz/dc. rc: the f64 offset's distance from the exact sample position.
    let (mut e, mut ed) = (0.0f64, 0.0f64);
    let rc = 16.0 * U * (ar.abs() + ai.abs()) + 1e-300;
    // The L1 norm bounds the Euclidean offset and avoids a square root per sample.
    let adc_bound = if L && D { ar.abs() + ai.abs() } else { 0.0 };
    let mut nucleus = if B { None } else { nucleus };
    let mut contraction = Contraction::default();
    while n < max_iter {
        // A block ends at the next Brent save point at the latest, so BLA saves (and so
        // judges periodicity against) exactly the iterates the plain kernel does.
        let block = match bla {
            // At phase zero A=0; use the actual pixel's 2z rather than that
            // reference-only derivative when measuring a nucleus return.
            Some(t) if L && !B && (nucleus.is_none() || m != 0) => t.find(
                m,
                xr * xr + xi * xi,
                max_iter.min(chk).min(nucleus.map_or(u64::MAX, |p| {
                    if n == 0 {
                        1
                    } else {
                        p as u64 + 1
                    }
                })) - n,
                MIN_PROFITABLE_STEPS,
            ),
            _ => None,
        };
        if let Some((b, len)) = block {
            // delta' = A delta + B dc, (dz/dc)' = A dz/dc + B.
            let ((a_r, a_i), (b_r, b_i)) = (b.a, b.b);
            if nucleus.is_some() && n >= 1 {
                contraction.multiply(a_r, a_i, 0);
            }
            if D {
                let t = a_r * dr - a_i * di + b_r;
                di = a_r * di + a_i * dr + b_i;
                dr = t;
                // Conservative first-order pixel-shift estimate: |v| <= |v.re|+|v.im|
                // and |d| >= max(|d.re|, |d.im|). Avoid two square roots per block.
                skip.shift +=
                    (b.alpha * (xr.abs() + xi.abs()) + b.beta * adc_bound) / dr.abs().max(di.abs());
            }
            let t = a_r * xr - a_i * xi + b_r * ar - b_i * ai;
            xi = a_r * xi + a_i * xr + b_r * ai + b_i * ar;
            xr = t;
            m += len;
            n += len as u64;
            skip.blocks += 1;
            skip.skipped += len as u64;
        } else {
            let (fr, fi) = (zr[m] + scale * xr, zi[m] + scale * xi);
            if nucleus.is_some() && n >= 1 {
                contraction.multiply(2.0 * fr, 2.0 * fi, 0);
            }
            if B {
                let af = (fr * fr + fi * fi).sqrt() * (1.0 + 4.0 * U) + 1e-150; // >= |f|, f = z as computed
                let ef = e + q[m] + 2.0 * U * af; // >= |z_exact - f|
                let ad = dr.abs() + di.abs();
                ed =
                    (2.0 * (af + ef) * ed + 2.0 * ef * ad + 8.0 * U * (2.0 * af * ad + 1.0)) * SAFE;
                let (aw, ax) = (zr[m].abs() + zi[m].abs(), xr.abs() + xi.abs());
                // Defect of the delta step against 2 A delta + delta^2 + dc_exact.
                let rho = 8.0 * U * (2.0 * aw * ax + ax * ax + ar.abs() + ai.abs())
                    + 2.0 * q[m] * ax
                    + rc;
                let w = af + q[m] + 2.0 * U * af; // >= |A_m + delta|
                e = (e * (2.0 * w + e) + rho) * SAFE;
            }
            if D {
                let t = 2.0 * (fr * dr - fi * di) + inverse;
                di = 2.0 * (fr * di + fi * dr);
                dr = t;
                if A::EXTENDED && dr.abs().max(di.abs()) > HI {
                    (dr, di, derivative_exp) = (dr * LO, di * LO, derivative_exp + 32);
                    inverse = exp2i(-derivative_exp);
                }
            }
            // delta' = 2 Z delta + delta^2 + dc
            (xr, xi) = A::delta((zr[m], zi[m]), (xr, xi), scale, (dc_r, dc_i));
            m += 1;
            n += 1;
        }
        let (physical_r, physical_i) = (scale * xr, scale * xi);
        let (fr, fi) = (zr[m] + physical_r, zi[m] + physical_i);
        let f2 = fr * fr + fi * fi;
        if f2 > r2 {
            let (ez, ed) = if B {
                (e + q[m] + 4.0 * U * f2.sqrt(), ed)
            } else {
                (f64::INFINITY, f64::INFINITY)
            };
            return Outcome::Escaped {
                n,
                zr: fr,
                zi: fi,
                dr,
                di,
                dexp: derivative_exp,
                ez,
                ed,
            };
        }
        if nucleus.is_some_and(|p| n == p as u64 + 1) {
            if contraction.small() {
                return Outcome::Interior { n };
            }
            nucleus = None; // no contraction: keep the ordinary escape/periodicity path
        }
        let delta2 = physical_r * physical_r + physical_i * physical_i;
        let rebase = if A::EXTENDED {
            f2 <= delta2
        } else {
            f2 < delta2
        };
        if rebase || m == last {
            if B {
                e += q[m] + 4.0 * U * f2.sqrt() + 1e-300; // delta := f, rounded, against A_m
            }
            if A::EXTENDED {
                (xr, xi, delta_exp) = if f2 > SQUARE_OK {
                    (fr, fi, 0)
                } else {
                    sum2(&[(zr[m], zi[m], 0), (xr, xi, delta_exp)], delta_exp)
                };
                scale = exp2i(delta_exp);
                m = 0;
                if scale < TINY {
                    // This exact-range step bypasses the ordinary product update;
                    // decline the heuristic rather than omit a derivative factor.
                    nucleus = None;
                    // The step from Z_0 needs exponent arithmetic when its square
                    // and dc cannot be represented as f64 (FIX-40).
                    if D {
                        (dr, di, derivative_exp) =
                            zero_step_dz((xr, xi, delta_exp), (dr, di, derivative_exp));
                        inverse = exp2i(-derivative_exp);
                    }
                    (xr, xi, delta_exp) = zero_step((xr, xi, delta_exp), (ar, ai, offset_exp));
                    scale = exp2i(delta_exp);
                    (m, n) = (1, n + 1);
                }
                let factor = exp2i(offset_exp - delta_exp);
                (dc_r, dc_i) = (ar * factor, ai * factor);
            } else {
                (xr, xi, m) = (fr, fi, 0); // rebase onto Z_0 = 0
            }
        } else if A::EXTENDED {
            let magnitude = xr.abs().max(xi.abs());
            let k = if magnitude > HI && delta_exp < 0 {
                32.min(-delta_exp)
            } else if magnitude < LO && magnitude != 0.0 {
                -32
            } else {
                0
            };
            if k != 0 {
                let factor = exp2i(-k);
                (xr, xi, delta_exp) = (xr * factor, xi * factor, delta_exp + k);
                scale = exp2i(delta_exp);
                let factor = exp2i(offset_exp - delta_exp);
                (dc_r, dc_i) = (ar * factor, ai * factor);
            }
        }
        // Until the delta is resolvable next to Z, z == Z in f64 and a periodic
        // reference would look like a settled cycle: judge nothing yet.
        // Ordinary deltas are tested after rebasing; the extended contract uses
        // the physical delta captured before rebase/rescale, as before.
        let (check_r, check_i) = if A::EXTENDED {
            (physical_r, physical_i)
        } else {
            (xr, xi)
        };
        let judge_cycle = if A::EXTENDED {
            returns < 4 && resolvable(check_r, check_i, f2)
        } else {
            resolvable(check_r, check_i, f2)
        };
        if !judge_cycle {
            if n == chk || (A::EXTENDED && n > chk) {
                (sr, si, sn, chk) = (fr, fi, n, chk * 2);
            }
            continue;
        }
        let dist = (fr - sr).abs() + (fi - si).abs();
        let size = sr.abs() + si.abs();
        if returns < 4 && dist < 1e-13 * size + 1e-300 {
            // A near-exact return is interior only if the orbit contracts over it (a
            // settled attracting cycle): an exterior orbit shadowing a repelling cycle
            // (near a minibrot) returns as closely, but expands (FIX-02).
            let dc_scale = if A::EXTENDED { exp2i(offset_exp) } else { 1.0 };
            if contracting(r, m, check_r, check_i, ar * dc_scale, ai * dc_scale, n - sn) {
                return if A::EXTENDED {
                    Outcome::Unresolved
                } else {
                    Outcome::Interior { n }
                };
            }
            returns += 1;
        }
        if !A::EXTENDED && tries < 4 && n >= newton_at && dist < 1e-3 * size {
            if attracting(r, m, xr, xi, ar, ai, n - sn) {
                return Outcome::Interior { n };
            }
            tries += 1;
            newton_at = 4 * n;
        }
        if n == chk || (A::EXTENDED && n > chk) {
            (sr, si, sn, chk) = (fr, fi, n, chk * 2);
        }
    }
    Outcome::Unresolved
}

/// Derivative of one nucleus return with respect to its initial state (not c).
/// DEC-10: heuristic |dF^P/dz| < 1e-3, only after the reference's nucleus gate;
/// otherwise ordinary iteration continues. Bounded-evidence renders decline it.
#[derive(Default)]
struct Contraction {
    re: f64,
    im: f64,
    exp: i64,
    started: bool,
}
impl Contraction {
    #[inline]
    fn multiply(&mut self, mut re: f64, mut im: f64, mut exp: i64) {
        if !self.started {
            self.re = 1.0;
            self.started = true;
        }
        let a = re.abs().max(im.abs());
        if a != 0.0 && (a > HI || a < LO) {
            let e = a.log2().floor() as i64;
            let scale = exp2i(-e);
            re *= scale;
            im *= scale;
            exp += e;
        }
        (self.re, self.im) = (self.re * re - self.im * im, self.re * im + self.im * re);
        self.exp += exp;
        let a = self.re.abs().max(self.im.abs());
        if a != 0.0 && (a > HI || a < LO) {
            let e = a.log2().floor() as i64;
            let scale = exp2i(-e);
            self.re *= scale;
            self.im *= scale;
            self.exp += e;
        }
    }
    fn small(&self) -> bool {
        self.started
            && self.re.is_finite()
            && self.im.is_finite()
            && (self.exp < -64
                || (self.exp <= 64
                    && (self.re * self.re + self.im * self.im) * exp2i(2 * self.exp) < 1e-6))
    }
}

/// Periodicity is judged only once `|delta| > RESOLVABLE |z|` ([`resolvable`]); BLA's
/// eps cap ([`crate::bla::EPS_MAX`]) is tied to it.
pub(crate) const RESOLVABLE: f64 = 1e-12;

/// Delta large enough relative to `|z|^2 = f2` that `z = Z + delta` carries it.
#[inline]
pub(crate) fn resolvable(xr: f64, xi: f64, f2: f64) -> bool {
    xr * xr + xi * xi > RESOLVABLE * RESOLVABLE * f2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_tier_render_bytes_are_unchanged() {
        use fd_samples::{Column, ColumnSet, View};
        let cases = [
            ("-0.7453", "0.1127", "0.0065", 0x172152a8483eca0e),
            ("0", "1", "1e-20", 0xf5e62b01cee54182),
            ("0", "1", "1e-300", 0xef98647c06aa30e1),
            ("0", "1", "1e-1000", 0xb93fa4cf0e38f4f0),
        ];
        for (re, im, width, expected) in cases {
            let view = View {
                center_re: re.into(),
                center_im: im.into(),
                width: width.into(),
                rotation: 0.3,
            };
            let params = crate::Params {
                nx: 8,
                ny: 4,
                ss: 1,
                max_iter: 5000,
                escape_radius: 1e10,
                columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
                threads: 1,
                tier: None,
            };
            let (header, samples) = crate::render(&view, &params).unwrap();
            let mut bytes = Vec::new();
            fd_samples::write(&mut bytes, &header, &samples).unwrap();
            let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
                (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
            });
            assert_eq!(hash, expected, "{width}: .fds bytes changed");
        }
    }

    fn run(cr: f64, ci: f64, max_iter: u64) -> Outcome {
        // Reference at a nearby point exercises the perturbation path.
        let r = Reference::new(cr - 1e-3, ci + 1e-3, max_iter, 1e10);
        sample::<true, false>(&r, &[], Some((cr, ci)), 1e-3, -1e-3, max_iter, 1e20)
    }

    fn direct(cr: f64, ci: f64, max_iter: u64) -> Option<u64> {
        let (mut x, mut y) = (0.0f64, 0.0f64);
        for n in 1..=max_iter {
            (x, y) = (x * x - y * y + cr, 2.0 * x * y + ci);
            if x * x + y * y > 1e20 {
                return Some(n);
            }
        }
        None
    }

    #[test]
    fn escape_count_matches_direct_iteration() {
        for (cr, ci) in [
            (0.4, 0.3),
            (-0.75, 0.1),
            (-1.8, 0.01),
            (0.2501, 0.0),
            (-0.1, 0.9),
            (-0.7487, 0.0789),
        ] {
            match run(cr, ci, 100_000) {
                Outcome::Escaped { n, .. } => {
                    assert_eq!(Some(n), direct(cr, ci, 100_000), "{cr},{ci}")
                }
                o => panic!("{cr},{ci}: {o:?}"),
            }
        }
    }

    #[test]
    fn interior_outside_main_components_is_found() {
        // Period-3 bulb centre region (real axis) and a period-4 bulb.
        for (cr, ci) in [(-1.7548, 0.0), (-0.1565, 1.0322), (-1.3107, 0.0)] {
            assert!(
                matches!(run(cr, ci, 1_000_000), Outcome::Interior { .. }),
                "{cr},{ci}"
            );
        }
    }

    #[test]
    fn derivative_is_skipped_without_changing_escape() {
        let r = Reference::new(0.3, 0.6, 1000, 1e10);
        let a = sample::<true, false>(&r, &[], Some((0.31, 0.6)), 0.01, 0.0, 1000, 1e20);
        let b = sample::<false, false>(&r, &[], Some((0.31, 0.6)), 0.01, 0.0, 1000, 1e20);
        match (a, b) {
            (Outcome::Escaped { n: x, .. }, Outcome::Escaped { n: y, .. }) => assert_eq!(x, y),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn bla_blocks_keep_the_escape_count() {
        // Parabolic slow escape near the cusp: deltas stay tiny for hundreds of steps.
        let r = Reference::new(0.2501, 0.0, 100_000, 1e10);
        let bla = Bla::build(&r, crate::bla::EPS_MAX, 2e-17).unwrap();
        for (ar, ai) in [(1e-17, 0.0), (-1e-17, 1e-17), (0.0, -1.4e-17)] {
            let plain = sample::<true, false>(&r, &[], None, ar, ai, 100_000, 1e20);
            let mut skip = Skip::default();
            let fast = perturb::<true, false, true>(
                &r,
                &[],
                Some(&bla),
                &mut skip,
                None,
                ar,
                ai,
                100_000,
                1e20,
                r.nucleus_period(),
            );
            match (plain, fast) {
                (
                    Outcome::Escaped { n, zr, dr, .. },
                    Outcome::Escaped {
                        n: n2,
                        zr: z2,
                        dr: d2,
                        ..
                    },
                ) => {
                    assert_eq!(n, n2, "{ar},{ai}");
                    assert!(
                        (zr - z2).abs() < 1e-6 * zr.abs() && (dr - d2).abs() < 1e-6 * dr.abs(),
                        "{zr} {z2} {dr} {d2}"
                    );
                }
                o => panic!("{o:?}"),
            }
            assert!(
                skip.skipped > 10 * skip.blocks && skip.blocks > 0,
                "{skip:?}"
            );
        }
    }

    /// Centre of the Atlas v0 path (bench/path-atlas-v0.txt): it sits close to a
    /// minibrot, so its reference orbit nearly repeats (period 122 and longer).
    const V0: (&str, &str) = (
        "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502",
        "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922",
    );

    /// Reference at the v0 centre in `bits` fixed point, and the offset of sample
    /// `(i, j)` of a 960x540 grid `width` wide.
    fn v0(bits: u64, iter: u64, width: f64, i: u32, j: u32) -> (Reference, f64, f64) {
        let l = fd_fixed::limbs_for(bits);
        let (re, im) = (
            fd_fixed::Fixed::parse(V0.0, l).unwrap(),
            fd_fixed::Fixed::parse(V0.1, l).unwrap(),
        );
        let h = width / 960.0;
        (
            Reference::from_fixed(&re, &im, iter),
            h * (f64::from(i) + 0.5 - 480.0),
            -h * (f64::from(j) + 0.5 - 270.0),
        )
    }

    #[test]
    fn a_near_return_that_expands_is_not_interior() {
        // FIX-02: frame 375 of the v0 path, sample (287, 212). At n = 378 the orbit is
        // back within 1e-13 of its iterate 256 (it shadows a repelling 122-cycle), which
        // the exact-return test took for a settled cycle; mpmath (tools/oracle.py's
        // direct iteration) has it escape at n = 788.
        let (r, ar, ai) = v0(218, 2000, 8.29151e-25, 287, 212);
        match sample::<true, false>(&r, &[], None, ar, ai, 2000, 1e20) {
            Outcome::Escaped { n, .. } => assert_eq!(n, 788),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn bla_judges_periodicity_at_the_plain_save_points() {
        // FIX-02: blocks used to land past the Brent save points (iterate 513 for 512),
        // so BLA judged periodicity against other iterates than the plain kernel and
        // could class a sample differently. Here (reference at the period-3 nucleus,
        // deltas tiny for thousands of steps) BLA said Interior at n = 36 where plain
        // iteration stays Unresolved. Blocks now end at the save points: the same
        // judgements at the same iterates, so the same outcome.
        let (cr, ci) = (-1.7548776662466927, 0.0);
        let r = Reference::new(cr, ci, 100_000, 1e10);
        for e in [1e-16, 1e-18] {
            let bla = Bla::build(&r, 1.0 / (1u64 << 50) as f64, 2.0 * e).unwrap();
            for k in 0..8 {
                let a = std::f64::consts::FRAC_PI_4 * f64::from(k);
                let (ar, ai) = (e * a.cos(), e * a.sin());
                let c = Some((cr + ar, ci + ai));
                let plain = sample::<true, false>(&r, &[], c, ar, ai, 100_000, 1e20);
                let mut skip = Skip::default();
                let fast = perturb::<true, false, true>(
                    &r,
                    &[],
                    Some(&bla),
                    &mut skip,
                    c,
                    ar,
                    ai,
                    100_000,
                    1e20,
                    r.nucleus_period(),
                );
                assert_eq!(plain, fast, "{e} {k}");
                assert!(skip.skipped > 10_000, "{e} {k}: {skip:?}");
            }
        }
        // The frame-749 sample (324, 10) of the v0 path (width 2e-49), which BLA classed
        // Interior at n = 6475 (a near-return to its iterate 4097, a save point the plain
        // kernel never takes): mpmath and the plain kernel have it escape at n = 7412.
        let (r, ar, ai) = v0(300, 20_000, 2e-49, 324, 10);
        let bla = Bla::build(&r, 1.0 / (1u64 << 50) as f64, 2e-49 * 1.15).unwrap();
        let mut skip = Skip::default();
        match perturb::<true, false, true>(
            &r,
            &[],
            Some(&bla),
            &mut skip,
            None,
            ar,
            ai,
            20_000,
            1e20,
            r.nucleus_period(),
        ) {
            Outcome::Escaped { n, .. } => assert_eq!(n, 7412),
            o => panic!("{o:?}"),
        }
        assert!(skip.skipped > 4096, "{skip:?}");
    }

    #[test]
    fn error_radius_covers_the_exact_orbit() {
        // Dyadic reference + offset, so the sample point is exact; compare against a
        // 256-bit fixed-point orbit at that point (f64 and fixed references).
        let l = fd_fixed::limbs_for(256);
        let fx = |s: &str| fd_fixed::Fixed::parse(s, l).unwrap();
        let cases = [
            (
                (0.375, 0.28125),
                (0.03125, 0.015625),
                ("0.40625", "0.296875"),
            ),
            (
                (-0.7421875, 0.0546875),
                (-0.0078125, 0.0078125),
                ("-0.75", "0.0625"),
            ),
            ((0.25, 0.0), (0.03125, 0.0078125), ("0.28125", "0.0078125")),
        ];
        let r2 = 4294967296.0f64;
        for ((c0r, c0i), (ar, ai), (er, ei)) in cases {
            let f64_ref = Reference::new(c0r, c0i, 10_000, r2.sqrt());
            let fx_ref =
                Reference::from_fixed(&fx(&c0r.to_string()), &fx(&c0i.to_string()), 10_000);
            let mut exact = vec![(0.0, 0.0)];
            fd_fixed::orbit(&fx(er), &fx(ei), 10_000, r2, |a, b| exact.push((a, b)));
            for (r, bits) in [(&f64_ref, None), (&fx_ref, Some(256))] {
                let q = r.error_radius(bits, c0r, c0i);
                let Outcome::Escaped {
                    n, zr, zi, ed, ez, ..
                } = sample::<true, true>(r, &q, Some((c0r + ar, c0i + ai)), ar, ai, 10_000, r2)
                else {
                    panic!("{er},{ei} did not escape")
                };
                let (xr, xi) = exact[n as usize];
                let (d, az) = ((zr - xr).hypot(zi - xi), zr.hypot(zi));
                assert!(
                    d <= ez + 4.0 * U * az,
                    "{er},{ei} {bits:?}: |dz| {d} > ez {ez}"
                );
                assert!(
                    ez < 1e-9 * az && ed.is_finite(),
                    "{er},{ei} {bits:?}: ez {ez} ed {ed} useless"
                );
            }
        }
    }
}

/// `sum t.0 2^t.2 + i t.1 2^t.2` as `(re, im, e)` with the larger part near unit size;
/// parts more than ~2^1000 below the largest are dropped. `(0, 0, zero_e)` if all vanish.
#[cold]
#[inline(never)]
fn sum2(terms: &[(f64, f64, i64)], zero_e: i64) -> (f64, f64, i64) {
    let lg = |t: &(f64, f64, i64)| {
        let a = t.0.abs().max(t.1.abs());
        (a != 0.0).then(|| a.log2().floor() as i64 + t.2)
    };
    let Some(top) = terms.iter().filter_map(lg).max() else {
        return (0.0, 0.0, zero_e);
    };
    // 2^k in two factors, so |k| up to 2046 stays finite.
    let sc = |x: f64, k: i64| {
        if k < -2046 {
            0.0
        } else {
            x * exp2i(k / 2) * exp2i(k - k / 2)
        }
    };
    let (re, im) = terms.iter().fold((0.0, 0.0), |(r, i), t| {
        (r + sc(t.0, t.2 - top), i + sc(t.1, t.2 - top))
    });
    (re, im, top)
}

/// `w' 2^e'` for the step from `Z_0 = 0`: `z' - Z_1 = z^2 + dc`, `z = 2^e w`, `dc = 2^ed a`.
#[cold]
#[inline(never)]
fn zero_step(w: (f64, f64, i64), a: (f64, f64, i64)) -> (f64, f64, i64) {
    let (wr, wi, e) = w;
    sum2(&[(wr * wr - wi * wi, 2.0 * wr * wi, 2 * e), a], e)
}

/// `v' 2^g'` for `dz/dc' = 2 z dz/dc + 1` with `z = 2^e w`, `dz/dc = 2^g v`.
#[cold]
#[inline(never)]
fn zero_step_dz(w: (f64, f64, i64), v: (f64, f64, i64)) -> (f64, f64, i64) {
    let q = (
        2.0 * (w.0 * v.0 - w.1 * v.1),
        2.0 * (w.0 * v.1 + w.1 * v.0),
        w.2 + v.2,
    );
    sum2(&[q, (1.0, 0.0, 0)], v.2)
}

#[cfg(test)]
mod exponent_tests {
    use super::*;
    use crate::sample::sample;
    fn at_exponent<const D: bool>(
        r: &Reference,
        ar: f64,
        ai: f64,
        exponent: i64,
        max_iter: u64,
        r2: f64,
    ) -> Outcome {
        iterate::<Binary64Exponent, D, false, false>(
            r,
            &[],
            None,
            &mut Skip::default(),
            None,
            ar,
            ai,
            exponent,
            max_iter,
            r2,
            r.nucleus_period(),
        )
    }

    /// The scaled kernel at an f64-representable offset must agree with the f64 kernel.
    #[test]
    fn agrees_with_f64_kernel_where_both_apply() {
        let r = Reference::new(-0.7487, 0.0789, 10_000, 1e10); // escapes after 52 steps
        for (ar, ai) in [(1e-5, 2e-5), (-3e-5, 1e-5), (5e-5, -5e-5)] {
            // Represent the same offset as 2^-20 * (ar * 2^20, ai * 2^20).
            let k = 1048576.0;
            let a = at_exponent::<true>(&r, ar * k, ai * k, -20, 10_000, 1e20);
            let b = sample::<true, false>(&r, &[], None, ar, ai, 10_000, 1e20);
            match (a, b) {
                (
                    Outcome::Escaped {
                        n: n1,
                        zr: z1,
                        dr: d1,
                        di: e1,
                        dexp,
                        ..
                    },
                    Outcome::Escaped {
                        n: n2,
                        zr: z2,
                        dr: d2,
                        di: e2,
                        ..
                    },
                ) => {
                    assert_eq!(n1, n2);
                    assert!((z1 - z2).abs() < 1e-6 * z2.abs());
                    let m1 = (d1 * d1 + e1 * e1).sqrt() * exp2i(dexp);
                    let m2 = (d2 * d2 + e2 * e2).sqrt();
                    assert!((m1 - m2).abs() < 1e-6 * m2, "{m1} {m2}");
                }
                o => panic!("{o:?}"),
            }
        }
    }

    /// Below f64 range the delta still grows and the sample escapes.
    #[test]
    fn escapes_from_far_below_f64_range() {
        // c = i is Misiurewicz: its 2-cycle multiplier is 4*sqrt(2), so a delta grows by
        // ~1.25 bits per step and a 2^-3000 offset escapes after roughly 2400 steps.
        let r = Reference::new(0.0, 1.0, 100_000, 1e10);
        match at_exponent::<true>(&r, 1.0, 0.5, -3000, 100_000, 1e20) {
            Outcome::Escaped { n, dexp, .. } => {
                assert!(n > 1500 && n < 4000 && dexp > 2900, "{n} {dexp}")
            }
            o => panic!("{o:?}"),
        }
    }

    /// At sub-f64 widths, a near-repelling or near-parabolic exterior is not interior.
    #[test]
    fn deep_repelling_and_near_parabolic_exteriors() {
        // Direct mpmath at 1200 bits: c + 2^-1100 * (1 + 0.5i).
        for (cr, ci, oracle_n) in [(0.0, 1.0, 887), (0.250001, 0.0, 3145), (-0.75, 0.01, 320)] {
            let r = Reference::new(cr, ci, 20_000, 65536.0);
            match at_exponent::<true>(&r, 1.0, 0.5, -1100, 20_000, 1e20) {
                Outcome::Escaped { n, .. } => assert_eq!(n, oracle_n, "{cr}, {ci}"),
                Outcome::Unresolved => {} // no attraction certificate
                o => panic!("{cr}, {ci}: {o:?}"),
            }
        }
    }

    /// Samples inside the v0 ladder's 1e-100 minibrot may remain unresolved.
    #[test]
    fn deep_ladder_interior_stays_non_escaping() {
        // k = 409 nucleus from tools/research/misiurewicz/ladder_rungs.txt.
        let limbs = fd_fixed::limbs_for(1280);
        let cr = fd_fixed::Fixed::parse(
            "-0.743291890852430202931624325972510757176348558120774356370590210260840951728744990155474936011582556204309239952297545796655290288288531259645658145590048845370914634545066454",
            limbs,
        )
        .unwrap();
        let ci = fd_fixed::Fixed::parse(
            "0.131240552308797604770845906581478143077114151158591754835109820677855616894736385331957444256069258907400140765662344877761310383645126000894706155294661737082962412611433299571783",
            limbs,
        )
        .unwrap();
        let r = Reference::from_fixed(&cr, &ci, 6000);
        for (ar, ai) in [(1.0, 0.0), (0.0, 1.0)] {
            let result = at_exponent::<true>(&r, ar, ai, -1100, 6000, 1e20);
            assert!(
                matches!(result, Outcome::Interior { .. } | Outcome::Unresolved),
                "{result:?}"
            );
        }
    }

    /// A 1e-1000 pixel next to the k = 7676 ladder nucleus escapes: the steps where the
    /// reference sits on 0 must keep `2^e w^2`, `dc` and `dz/dc` below f64 range (FIX-40).
    #[test]
    fn deep_nucleus_neighbour_escapes_at_the_oracle_count() {
        let (_, _, r) = rung_7676(140_000);
        // Pixel (5, 5) of a 96x54 frame 5000 minibrot sizes wide; mpmath at 1200
        // digits: escape (|z| > 1e10) at n = 133115, z = -2.7519423631080235e10 - ...,
        // log2 |dz/dc| = 3359.4391791173814.
        match at_exponent::<true>(
            &r,
            -0.5737130423454617,
            0.290231303774763,
            -3310,
            140_000,
            1e20,
        ) {
            Outcome::Escaped {
                n,
                zr,
                dr,
                di,
                dexp,
                ..
            } => {
                assert_eq!(n, 133_115);
                assert!((zr / -27519423631.080235 - 1.0).abs() < 1e-6, "{zr}");
                let l = dr.hypot(di).log2() + dexp as f64;
                assert!((l - 3359.4391791173814).abs() < 1e-6, "{l}");
            }
            o => panic!("{o:?}"),
        }
        // Pixel (48, 19), near the nucleus: after its eighth return |z| ~ 2^-557 is above
        // the old 1e-270 cut but its square is not, so a rebase to exponent 0 rounded
        // z^2 to 0 and the pixel never escaped. mpmath: escape at n = 139850.
        match at_exponent::<true>(
            &r,
            0.006749565204064256,
            0.10124347806096384,
            -3310,
            140_000,
            1e20,
        ) {
            Outcome::Escaped { n, .. } => assert_eq!(n, 139_850),
            o => panic!("{o:?}"),
        }
    }

    /// The k = 7676 ladder nucleus (~1e-1000 minibrot) as decimal strings and a
    /// reference of length `len`.
    fn rung_7676(len: u64) -> (String, String, Reference) {
        let rung = include_str!("../../../tools/research/misiurewicz/ladder_rungs.txt")
            .lines()
            .find(|line| line.starts_with("7676 "))
            .unwrap();
        let f: Vec<&str> = rung.split_whitespace().collect();
        let limbs = fd_fixed::limbs_for(3456);
        let cr = fd_fixed::Fixed::parse(f[2], limbs).unwrap();
        let ci = fd_fixed::Fixed::parse(f[3], limbs).unwrap();
        (
            f[2].to_string(),
            f[3].to_string(),
            Reference::from_fixed(&cr, &ci, len),
        )
    }

    /// PROB-19's k = 7676 frame 500 minibrot sizes wide (480x270), pixel (184, 201):
    /// the orbit shadows the weakly repelling M(24,2) 2-cycle (|rho| ~ 1.15) to within
    /// 1e-13 for many laps, which the close-return check took for a settled cycle and
    /// returned Unresolved (FIX-41). mpmath at 1200 digits: escape at n = 145079.
    #[test]
    fn repelling_cycle_shadow_at_the_7676_rung_escapes() {
        let (re, im, r) = rung_7676(150_000);
        let v = fd_samples::View {
            center_re: re,
            center_im: im,
            width: "5.050000000000E-998".into(),
            rotation: 0.0,
        };
        let pl = crate::Plane::new(&v, 480, 270).unwrap();
        let (ux, uy) = pl.unit_offset(184, 201);
        match at_exponent::<true>(&r, ux * pl.h_m, uy * pl.h_m, pl.h_e, 644_640, 1e20) {
            Outcome::Escaped { n, .. } => assert_eq!(n, 145_079),
            o => panic!("{o:?}"),
        }
    }
}
