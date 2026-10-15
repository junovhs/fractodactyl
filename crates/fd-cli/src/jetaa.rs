//! Jet-AA for films (FILM-15; research: docs/research/10-11-26/q6-supersampling-from-jets.md).
//!
//! An ss 2 film frame is rendered in two kernel passes. Pass 1 computes each pixel's
//! top-left sample only. [`rebuild`] then predicts the other three samples of each pixel
//! from the stored columns of the pass-1 samples, and pass 2 computes only the predictions
//! it rejected. A pixel whose predictions are all rejected costs exactly the four samples
//! of the exact ss 2 render, never more.
//!
//! Films only: the bar is DEC-29 (viewers cannot tell a frame from the exact render).
//! Stills keep every sample exact (DEC-17).
//!
//! **Prediction.** F = 2^-n log z_n (the log-Boettcher coordinate) is holomorphic in the
//! screen coordinate s = dx - i dy (output pixels, any rotation) and continuous across
//! escape counts. A sample's columns give F up to one common scale: Re F ∝ 2^-nu,
//! |F'| = 2 Re F / de, and arg F' is the screen normal. The four neighbouring pixels' F'
//! (at s = +-1, +-i) give F'' and F''' by a stencil whose h^3 (F'') and h^4 (F''')
//! error terms cancel. From F, F', F'', F''' at a pixel's sample, nu, de and the normal
//! follow at the pixel's other three sample positions.
//!
//! **Consistency check.** Each predicted sample is also predicted from the neighbouring
//! pixel's sample on its other side (right, lower or diagonal). It is accepted only when
//! the two agree (|dnu| < M 1e-3, |d ln de| < M 1e-2, |d normal| < M 3e-3 rad), and then
//! takes their mean. Otherwise pass 2 computes it.
//!
//! Measured (research probe, nine looks, ss 2 exact render as reference): worst look ≤ 0.05%
//! of pixels over 1 level on every probed view and on 1080p v0 zone frames from 1e-29 to
//! 2e-49, no added flicker. 1.0 to 2.7 samples per pixel on most content, 2.9 to 3.6 on
//! the densest frames.
use fd_samples::{Class, Evidence, Kind, Samples};

/// Consistency tolerance multiplier (the research probe's m = 3).
const M: f64 = 3.0;
/// Pixels whose sample lies closer than this to the set (de, output px) are not predicted:
/// their predictions half a pixel away are rejected anyway (research probe), so skipping
/// them only saves time.
const DE_FLOOR: f32 = 0.3;

/// Pass-1 mask over the ss 2 sample grid of a `w` x `h` pixel frame: each pixel's top-left
/// sample.
pub(crate) fn first(w: usize, h: usize) -> Vec<bool> {
    (0..4 * w * h)
        .map(|i| (i % (2 * w)).is_multiple_of(2) && (i / (2 * w)).is_multiple_of(2))
        .collect()
}

#[derive(Clone, Copy)]
struct C(f64, f64);

impl C {
    fn add(self, o: C) -> C {
        C(self.0 + o.0, self.1 + o.1)
    }
    fn sub(self, o: C) -> C {
        C(self.0 - o.0, self.1 - o.1)
    }
    fn mul(self, o: C) -> C {
        C(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn scale(self, k: f64) -> C {
        C(self.0 * k, self.1 * k)
    }
}

/// Powers (d, d^2/2, d^3/6) of a screen offset d = dx - i dy.
#[derive(Clone, Copy)]
struct Off(C, C, C);

impl Off {
    fn new(dx: f64, dy: f64) -> Off {
        let d = C(dx, -dy);
        let d2 = d.mul(d);
        Off(d, d2.scale(0.5), d2.mul(d).scale(1.0 / 6.0))
    }
}

/// A pixel's jet: its sample's nu and F', F'', F''' with Re F scaled to 1.
#[derive(Clone, Copy)]
struct Jet {
    nu: f64,
    f1: C,
    f2: C,
    f3: C,
}

impl Jet {
    /// (nu, de, normal x, normal y) at the screen offset with powers `o` = (d, d^2/2,
    /// d^3/6) of d = dx - i dy (output pixels); see [`Off`].
    fn at(&self, o: &Off) -> Option<(f64, f32, f64, f64)> {
        let (d, d2h, d3s) = (o.0, o.1, o.2);
        let re = 1.0 + self.f1.mul(d).0 + self.f2.mul(d2h).0 + self.f3.mul(d3s).0;
        let fp = self.f1.add(self.f2.mul(d)).add(self.f3.mul(d2h));
        let de = 2.0 * re / (fp.0 * fp.0 + fp.1 * fp.1).sqrt();
        let nu = self.nu - re.log2();
        (re > 0.0 && nu.is_finite() && de.is_finite() && de > 0.0)
            .then_some((nu, de as f32, fp.0, fp.1))
    }
}

/// `f(i)` for `i` in `0..n` on all cores, in order, written in place.
fn par_map<T: Copy + Send>(n: usize, init: T, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get());
    let chunk = n.div_ceil(threads).max(1);
    let mut out = vec![init; n];
    std::thread::scope(|sc| {
        for (c, part) in out.chunks_mut(chunk).enumerate() {
            let f = &f;
            sc.spawn(move || {
                for (k, o) in part.iter_mut().enumerate() {
                    *o = f(c * chunk + k);
                }
            });
        }
    });
    out
}

/// A predicted sample: `None` to compute, `Some(None)` the pixel's own non-escaped class,
/// `Some(Some((nu, de, normal)))` an escaped sample.
type Pred = Option<Option<(f64, f32, u16)>>;

/// Rebuild the three unsampled positions of every pixel of `s` (an ss 2 grid of `w` x `h`
/// pixels whose top-left samples are computed) and return the mask of positions pass 2
/// must compute.
pub(crate) fn rebuild(s: &mut Samples, w: usize, h: usize) -> Vec<bool> {
    let nx = 2 * w;
    let idx = |px: usize, py: usize, k: usize| (2 * py + k / 2) * nx + 2 * px + k % 2;
    let real = |q: usize| idx(q % w, q / w, 0);
    let (nus, des, nms) = (
        s.nu.as_ref().unwrap(),
        s.de.as_ref().unwrap(),
        s.normal.as_ref().unwrap(),
    );
    // Per pixel sample: (nu, 2 F' / Re F) = (nu, 2 e^(i normal) / de), escaped only.
    let g: Vec<Option<(f64, C)>> = par_map(w * h, None, |q| {
        let i = real(q);
        (s.class[i].kind() == Some(Kind::Escaped)).then(|| {
            let (x, y) = Samples::unit(nms[i]);
            (
                nus[i],
                C(f64::from(x), f64::from(y)).scale(2.0 / f64::from(des[i])),
            )
        })
    });
    let jets: Vec<Option<Jet>> = par_map(w * h, None, |q| {
        let (px, py) = (q % w, q / w);
        if px == 0 || py == 0 || px + 1 >= w || py + 1 >= h {
            return None;
        }
        let (nu, f0) = g[q]?;
        // F' of a neighbour relative to Re F here: Re F scales as 2^-nu.
        let at = |r: usize| g[r].map(|(n, c)| c.scale((nu - n).exp2()));
        let (rt, lf, up, dn) = (at(q + 1)?, at(q - 1)?, at(q - w)?, at(q + w)?);
        // F' at s = 1, -1, i (up), -i (down)
        let a = rt.sub(lf);
        let b = up.sub(dn);
        let f2 = C(a.0 + b.1, a.1 - b.0).scale(0.25);
        let f3 = rt.add(lf).sub(up).sub(dn).scale(0.5);
        let j = Jet { nu, f1: f0, f2, f3 };
        [j.f2, j.f3]
            .iter()
            .all(|c| c.0.is_finite() && c.1.is_finite())
            .then_some(j)
    });
    // Offsets of samples 1..3 from the pixel's own sample and from the right, lower and
    // diagonal neighbour's sample.
    let own = [Off::new(0.5, 0.0), Off::new(0.0, 0.5), Off::new(0.5, 0.5)];
    let other = [
        Off::new(-0.5, 0.0),
        Off::new(0.0, -0.5),
        Off::new(-0.5, -0.5),
    ];
    let preds: Vec<[Pred; 3]> = par_map(w * h, [None; 3], |q| {
        let (px, py) = (q % w, q / w);
        let mut pred: [Pred; 3] = [None; 3];
        if px == 0 || py == 0 || px + 2 >= w || py + 2 >= h {
            return pred;
        }
        let r0 = real(q);
        if g[q].is_none() {
            // Inside: predict the sample's own class only where every neighbour is too.
            let kind = s.class[r0].kind();
            if [q + 1, q + w, q + w + 1, q - 1, q - w]
                .iter()
                .all(|&r| s.class[real(r)].kind() == kind)
            {
                pred = [Some(None); 3];
            }
            return pred;
        }
        if des[r0] < DE_FLOOR {
            return pred;
        }
        let Some(j) = jets[q] else { return pred };
        let nb = [q + 1, q + w, q + w + 1];
        for k in 1..4 {
            let (Some(a), Some(b)) = (
                j.at(&own[k - 1]),
                jets[nb[k - 1]].and_then(|jn| jn.at(&other[k - 1])),
            ) else {
                continue;
            };
            if (a.0 - b.0).abs() >= M * 1e-3 || f64::from(a.1 / b.1).ln().abs() >= M * 1e-2 {
                continue;
            }
            let (na, nb_) = (
                (a.2 * a.2 + a.3 * a.3).sqrt(),
                (b.2 * b.2 + b.3 * b.3).sqrt(),
            );
            let (ua, ub) = (C(a.2 / na, a.3 / na), C(b.2 / nb_, b.3 / nb_));
            // |angle| < M 3e-3 rad: same half-plane and small cross product
            let (cross, dot) = (ua.0 * ub.1 - ua.1 * ub.0, ua.0 * ub.0 + ua.1 * ub.1);
            if dot > 0.0 && cross.abs() < (M * 3e-3).sin() {
                pred[k - 1] = Some(Some((
                    0.5 * (a.0 + b.0),
                    0.5 * (a.1 + b.1),
                    Samples::angle(ua.0 + ub.0, ua.1 + ub.1),
                )));
            }
        }
        pred
    });
    let mut todo = vec![false; 4 * w * h];
    for (q, pred) in preds.iter().enumerate() {
        let (px, py) = (q % w, q / w);
        let inside = s.class[real(q)];
        for (k, p) in pred.iter().enumerate() {
            let i = idx(px, py, k + 1);
            match p {
                None => todo[i] = true,
                Some(None) => {
                    s.class[i] = inside
                        .kind()
                        .map_or(inside, |kind| Class::new(kind, Evidence::Heuristic))
                }
                Some(Some((nu, de, a))) => {
                    s.class[i] = Class::new(Kind::Escaped, Evidence::Heuristic);
                    s.nu.as_mut().unwrap()[i] = *nu;
                    s.de.as_mut().unwrap()[i] = *de;
                    s.normal.as_mut().unwrap()[i] = *a;
                }
            }
        }
    }
    todo
}

/// Copy the samples selected by `mask` from `from` into `to`.
pub(crate) fn merge(to: &mut Samples, from: &Samples, mask: &[bool]) {
    for (i, _) in mask.iter().enumerate().filter(|(_, &m)| m) {
        to.class[i] = from.class[i];
        if let (Some(a), Some(b)) = (to.nu.as_mut(), from.nu.as_ref()) {
            a[i] = b[i];
        }
        if let (Some(a), Some(b)) = (to.de.as_mut(), from.de.as_ref()) {
            a[i] = b[i];
        }
        if let (Some(a), Some(b)) = (to.normal.as_mut(), from.normal.as_ref()) {
            a[i] = b[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_kernel::{render, Params};
    use fd_samples::View;

    /// Exact ss 2 render of a seahorse-valley view; returns (samples, w, h).
    fn exact(rotation: f64) -> (Samples, usize, usize) {
        let (w, h) = (96usize, 54usize);
        let view = View {
            center_re: "-0.7453".into(),
            center_im: "0.1127".into(),
            width: "0.0065".into(),
            rotation,
        };
        let p = Params {
            nx: 2 * w as u32,
            ny: 2 * h as u32,
            ss: 2,
            max_iter: 20_000,
            escape_radius: 1e10,
            columns: crate::render::columns("nu,de,normal").unwrap(),
            threads: 2,
            tier: None,
        };
        let (_, s) = render(&view, &p).unwrap();
        (s, w, h)
    }

    /// Keep only the pass-1 samples of `s`, rebuild, and check every accepted prediction
    /// against the exact sample: (accepted share, worst |dnu|, worst relative dDE).
    fn check(rotation: f64) -> (f64, f64, f64) {
        let (ex, w, h) = exact(rotation);
        let keep = first(w, h);
        let mut s = ex.clone();
        for (i, k) in keep.iter().enumerate() {
            if !k {
                s.class[i] = Class(0);
            }
        }
        let todo = rebuild(&mut s, w, h);
        let (mut acc, mut dnu, mut dde) = (0usize, 0f64, 0f64);
        for i in 0..todo.len() {
            if keep[i] || todo[i] {
                continue;
            }
            acc += 1;
            assert_eq!(s.class[i].kind(), ex.class[i].kind(), "sample {i}");
            if ex.class[i].kind() == Some(Kind::Escaped) {
                dnu = dnu.max((s.nu.as_ref().unwrap()[i] - ex.nu.as_ref().unwrap()[i]).abs());
                let (a, b) = (s.de.as_ref().unwrap()[i], ex.de.as_ref().unwrap()[i]);
                dde = dde.max(f64::from((a - b).abs() / b));
            }
        }
        (acc as f64 / (3 * w * h) as f64, dnu, dde)
    }

    #[test]
    fn first_marks_each_pixels_top_left_sample() {
        let m = first(3, 2);
        assert_eq!(m.iter().filter(|&&b| b).count(), 6);
        assert!(m[0] && !m[1] && m[2] && !m[6] && m[12]);
    }

    #[test]
    fn accepted_predictions_match_the_exact_samples() {
        let (share, dnu, dde) = check(0.0);
        assert!(share > 0.25, "accepted {share}");
        assert!(dnu < 1e-2 && dde < 5e-2, "dnu {dnu} dde {dde}");
    }

    /// Rotated frames predict as accurately (measured: worst dnu 0.0028 / 0.0043 / 0.0029
    /// at rotations 0 / 0.7 / 2.0; a wrong axis convention would reject nearly everything).
    #[test]
    fn rotation_does_not_change_the_rebuild() {
        let (share, dnu, dde) = check(0.7);
        assert!(share > 0.25, "accepted {share}");
        assert!(dnu < 1e-2 && dde < 5e-2, "dnu {dnu} dde {dde}");
    }
}

#[cfg(test)]
mod bench {
    /// `cargo test --release -p fd-cli jetaa::bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn rebuild_time() {
        let (w, h) = (960usize, 540usize);
        let view = fd_samples::View {
            center_re: "-0.7453".into(),
            center_im: "0.1127".into(),
            width: "0.0065".into(),
            rotation: 0.0,
        };
        let p = fd_kernel::Params {
            nx: 2 * w as u32,
            ny: 2 * h as u32,
            ss: 2,
            max_iter: 2000,
            escape_radius: 1e10,
            columns: crate::render::columns("nu,de,normal").unwrap(),
            threads: 4,
            tier: None,
        };
        let (_, s) = fd_kernel::render(&view, &p).unwrap();
        for _ in 0..3 {
            let mut c = s.clone();
            let t = std::time::Instant::now();
            let todo = super::rebuild(&mut c, w, h);
            println!(
                "rebuild {:.3} s, todo {}",
                t.elapsed().as_secs_f64(),
                todo.iter().filter(|&&b| b).count()
            );
        }
    }
}
