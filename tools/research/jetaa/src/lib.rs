//! Q6 jet-AA probe: perturbation samples that carry their Taylor jet in c.
//!
//! Derivatives are taken with respect to `u`, the parameter offset in output pixels
//! (c = c_pixel + hp * u), so they stay in f64 range at the depths probed here
//! (d1 starts at hp, d3 at ~hp^3; keep hp above ~1e-95).
//!
//! Every sample's raw record is `n, zr, zi, d1r, d1i[, d2r, d2i, d3r, d3i]`, with
//! `n = 0` for a sample that did not escape within `max_iter` (unresolved).
use std::io::{Read, Write};

pub const ESCAPE2: f64 = 1e20; // escape radius 1e10, as fd films use

/// Reference orbit as hi + lo doubles.
pub struct Ref {
    pub rh: Vec<f64>,
    pub rl: Vec<f64>,
    pub ih: Vec<f64>,
    pub il: Vec<f64>,
}

impl Ref {
    pub fn load(path: &str) -> Result<Ref, String> {
        let mut b = Vec::new();
        std::fs::File::open(path)
            .and_then(|mut f| f.read_to_end(&mut b))
            .map_err(|e| format!("{path}: {e}"))?;
        if &b[0..8] != b"JREF0001" {
            return Err("bad reference magic".into());
        }
        let n = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        let f = |k: usize| f64::from_le_bytes(b[12 + 8 * k..20 + 8 * k].try_into().unwrap());
        let mut r = Ref { rh: vec![], rl: vec![], ih: vec![], il: vec![] };
        for i in 0..n {
            r.rh.push(f(4 * i));
            r.rl.push(f(4 * i + 1));
            r.ih.push(f(4 * i + 2));
            r.il.push(f(4 * i + 3));
        }
        Ok(r)
    }
    pub fn len(&self) -> usize {
        self.rh.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rh.is_empty()
    }
}

/// One binary64 sample (production arithmetic) at parameter offset `dc` (plane units) from
/// the reference, with `ORDER` derivatives in u (1 or 3). Returns the raw record.
#[inline(never)]
pub fn sample<const ORDER: usize>(r: &Ref, dcr: f64, dci: f64, hp: f64, max_iter: u32) -> [f64; 9] {
    let (mut er, mut ei) = (0.0f64, 0.0f64); // delta
    let (mut ar, mut ai) = (0.0f64, 0.0f64); // d1
    let (mut br, mut bi) = (0.0f64, 0.0f64); // d2
    let (mut gr, mut gi) = (0.0f64, 0.0f64); // d3
    let mut m = 0usize;
    let last = r.len() - 1;
    for it in 1..=max_iter {
        let (zr, zi) = (r.rh[m] + er, r.ih[m] + ei);
        if ORDER >= 3 {
            // d3' = 2 (z d3 + 3 d1 d2), d2' = 2 (z d2 + d1^2)
            let (pr, pi) = (ar * br - ai * bi, ar * bi + ai * br);
            let ngr = 2.0 * (zr * gr - zi * gi + 3.0 * pr);
            let ngi = 2.0 * (zr * gi + zi * gr + 3.0 * pi);
            let nbr = 2.0 * (zr * br - zi * bi + ar * ar - ai * ai);
            let nbi = 2.0 * (zr * bi + zi * br + 2.0 * ar * ai);
            gr = ngr;
            gi = ngi;
            br = nbr;
            bi = nbi;
        }
        let nar = 2.0 * (zr * ar - zi * ai) + hp;
        let nai = 2.0 * (zr * ai + zi * ar);
        ar = nar;
        ai = nai;
        // delta' = (2 Z + delta) delta + dc
        let (tr, ti) = (2.0 * r.rh[m] + er, 2.0 * r.ih[m] + ei);
        let ner = tr * er - ti * ei + dcr;
        let nei = tr * ei + ti * er + dci;
        er = ner;
        ei = nei;
        m += 1;
        let (zr, zi) = (r.rh[m] + er, r.ih[m] + ei);
        let z2 = zr * zr + zi * zi;
        if z2 > ESCAPE2 {
            return [it as f64, zr, zi, ar, ai, br, bi, gr, gi];
        }
        if z2 < er * er + ei * ei || m == last {
            er = zr;
            ei = zi;
            m = 0;
        }
    }
    [0.0; 9]
}

// ---- double-double -------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Dd(pub f64, pub f64);

#[inline]
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}
#[inline]
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    (s, b - (s - a))
}
impl Dd {
    #[inline]
    pub fn add(self, o: Dd) -> Dd {
        let (s, e) = two_sum(self.0, o.0);
        let (t, f) = two_sum(self.1, o.1);
        let (s, e) = fast_two_sum(s, e + t);
        let (s, e) = fast_two_sum(s, e + f);
        Dd(s, e)
    }
    #[inline]
    pub fn neg(self) -> Dd {
        Dd(-self.0, -self.1)
    }
    #[inline]
    pub fn sub(self, o: Dd) -> Dd {
        self.add(o.neg())
    }
    #[inline]
    pub fn mul(self, o: Dd) -> Dd {
        let p = self.0 * o.0;
        let e = self.0.mul_add(o.0, -p);
        let e = e + (self.0 * o.1 + self.1 * o.0);
        let (s, e) = fast_two_sum(p, e);
        Dd(s, e)
    }
    #[inline]
    pub fn mul2(self) -> Dd {
        Dd(2.0 * self.0, 2.0 * self.1)
    }
    #[inline]
    pub fn from_prod(a: f64, b: f64) -> Dd {
        let p = a * b;
        Dd(p, a.mul_add(b, -p))
    }
}

/// The same sample with the perturbation delta, the reference and dc in double-double
/// (the truth arm near edges). d1 stays binary64 (it only feeds DE and the normal).
#[inline(never)]
pub fn sample_dd(r: &Ref, dcr: Dd, dci: Dd, hp: f64, max_iter: u32) -> [f64; 9] {
    let (mut er, mut ei) = (Dd(0.0, 0.0), Dd(0.0, 0.0));
    let (mut ar, mut ai) = (0.0f64, 0.0f64);
    let mut m = 0usize;
    let last = r.len() - 1;
    for it in 1..=max_iter {
        let (zrr, zir) = (Dd(r.rh[m], r.rl[m]), Dd(r.ih[m], r.il[m]));
        let (zr, zi) = (zrr.add(er), zir.add(ei));
        let nar = 2.0 * (zr.0 * ar - zi.0 * ai) + hp;
        let nai = 2.0 * (zr.0 * ai + zi.0 * ar);
        ar = nar;
        ai = nai;
        let (tr, ti) = (zrr.mul2().add(er), zir.mul2().add(ei));
        let ner = tr.mul(er).sub(ti.mul(ei)).add(dcr);
        let nei = tr.mul(ei).add(ti.mul(er)).add(dci);
        er = ner;
        ei = nei;
        m += 1;
        let (zr, zi) = (Dd(r.rh[m], r.rl[m]).add(er), Dd(r.ih[m], r.il[m]).add(ei));
        let z2 = zr.0 * zr.0 + zi.0 * zi.0;
        if z2 > ESCAPE2 {
            return [it as f64, zr.0, zi.0, ar, ai, 0.0, 0.0, 0.0, 0.0];
        }
        if z2 < er.0 * er.0 + ei.0 * ei.0 || m == last {
            er = zr;
            ei = zi;
            m = 0;
        }
    }
    [0.0; 9]
}

// ---- shading inputs ---------------------------------------------------------------------

/// Production columns from a raw record: (nu, de in output px, screen normal (x, y)).
/// `None` for an unresolved sample.
#[inline]
pub fn columns(raw: &[f64]) -> Option<(f64, f32, f64, f64)> {
    if raw[0] == 0.0 {
        return None;
    }
    let (n, zr, zi, dr, di) = (raw[0], raw[1], raw[2], raw[3], raw[4]);
    let z2 = zr * zr + zi * zi;
    let log2z = 0.5 * z2.log2();
    let nu = n + 1.0 - log2z.log2();
    let az = z2.sqrt();
    let de = (2.0 * az * log2z * std::f64::consts::LN_2 / dr.hypot(di)) as f32;
    // direction of z * conj(dz/du), to screen (y down)
    let (x, y) = (zr * dr + zi * di, -(zi * dr - zr * di));
    Some((nu, de, x, y))
}

/// The jet of log z_n at a sample: ln|z|, w = z'/z, w' = (z''/z) - w^2, w'' (complex pairs).
#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub n: f64,
    pub l: f64,
    pub w: [f64; 2],
    pub w1: [f64; 2],
    pub w2: [f64; 2],
}

#[inline]
fn cmul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]]
}
#[inline]
fn cdiv(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let d = b[0] * b[0] + b[1] * b[1];
    [(a[0] * b[0] + a[1] * b[1]) / d, (a[1] * b[0] - a[0] * b[1]) / d]
}

impl Jet {
    pub fn from_raw(raw: &[f64]) -> Option<Jet> {
        if raw[0] == 0.0 {
            return None;
        }
        let z = [raw[1], raw[2]];
        let a = cdiv([raw[3], raw[4]], z);
        let b = cdiv([raw[5], raw[6]], z);
        let g = cdiv([raw[7], raw[8]], z);
        let w = a;
        let ww = cmul(w, w);
        let w1 = [b[0] - ww[0], b[1] - ww[1]];
        // w'' = z'''/z - 3 w z''/z + 2 w^3
        let wb = cmul(w, b);
        let www = cmul(ww, w);
        let w2 = [g[0] - 3.0 * wb[0] + 2.0 * www[0], g[1] - 3.0 * wb[1] + 2.0 * www[1]];
        Some(Jet { n: raw[0], l: 0.5 * (z[0] * z[0] + z[1] * z[1]).ln(), w, w1, w2 })
    }

    /// Columns at screen offset (dx, dy) output pixels from the sample, from a jet of
    /// `order` (1: w only, 2: + w', 3: + w''). The value jet uses one order more than the
    /// derivative jet (`ln|z|` to `order`, `w` to `order - 1`).
    #[inline]
    pub fn at(&self, dx: f64, dy: f64, order: u32) -> (f64, f32, f64, f64) {
        let d = [dx, -dy]; // u offset: screen y is -im
        let d2 = cmul(d, d);
        let d3 = cmul(d2, d);
        let mut l = self.l + (self.w[0] * d[0] - self.w[1] * d[1]);
        let mut wv = self.w;
        if order >= 2 {
            let t = cmul(self.w1, d2);
            l += 0.5 * t[0];
            let t = cmul(self.w1, d);
            wv = [wv[0] + t[0], wv[1] + t[1]];
        }
        if order >= 3 {
            let t = cmul(self.w2, d3);
            l += t[0] / 6.0;
            let t = cmul(self.w2, d2);
            wv = [wv[0] + 0.5 * t[0], wv[1] + 0.5 * t[1]];
        }
        let log2z = l / std::f64::consts::LN_2;
        let nu = self.n + 1.0 - log2z.log2();
        let de = (2.0 * l / (wv[0].hypot(wv[1]))) as f32;
        // z conj(z') = |z|^2 conj(w): direction conj(w), to screen (re, -im) = (w_r, w_i)
        (nu, de, wv[0], wv[1])
    }
}

/// Second- and third-order jet terms from the first-order data (z, dz/du) of the four
/// neighbouring pixel centres, one output pixel away: no extra recurrences.
///
/// F = 2^-n log z_n is the log-Boettcher coordinate: holomorphic in u and continuous
/// across escape counts, so each neighbour's F' = w / 2^n_k is comparable. With F' known
/// at u = +-1 and +-i (screen y down: the pixel below is u = -i):
///   F2 = [(F'(1) - F'(-1)) - i (F'(i) - F'(-i))] / 4    (error O(F^(6)))
///   F3 = [F'(1) + F'(-1) - F'(i) - F'(-i)] / 2          (the h^4 terms cancel: O(F^(7)))
/// Returned in the centre's units (times 2^n): the centre jet with w1, w2 replaced.
/// `nb` = raw records of [right, left, up, down]; `None` if any is unresolved.
pub fn jet_from_neighbours(centre: &[f64], nb: [&[f64]; 4]) -> Option<Jet> {
    let mut j = Jet::from_raw(centre)?;
    let n0 = centre[0];
    let mut f = [[0.0f64; 2]; 4];
    for (k, r) in nb.iter().enumerate() {
        if r[0] == 0.0 {
            return None;
        }
        let w = cdiv([r[3], r[4]], [r[1], r[2]]);
        let s = (n0 - r[0]).exp2();
        f[k] = [w[0] * s, w[1] * s];
    }
    let [rt, lf, up, dn] = f; // F' at u = 1, -1, i, -i
    let a = [rt[0] - lf[0], rt[1] - lf[1]];
    let b = [up[0] - dn[0], up[1] - dn[1]]; // times -i: (x, y) -> (y, -x)
    j.w1 = [(a[0] + b[1]) / 4.0, (a[1] - b[0]) / 4.0];
    j.w2 = [(rt[0] + lf[0] - up[0] - dn[0]) / 2.0, (rt[1] + lf[1] - up[1] - dn[1]) / 2.0];
    Some(j)
}

/// A raw record `n, z, dz/du` rebuilt from stored columns only (nu, de in output px, normal
/// angle): ln|z| from nu (taking n = floor(nu)), |w| = 2 ln|z| / de along the normal.
/// Enough for the neighbour stencil and `Jet::at` (which never read arg z).
pub fn raw_from_columns(nu: f64, de: f32, angle: u16) -> [f64; 9] {
    let n = nu.floor();
    let l = std::f64::consts::LN_2 * (n + 1.0 - nu).exp2();
    let (c, s) = fd_samples::Samples::unit(angle);
    let aw = 2.0 * l / de as f64;
    let ez = l.exp();
    [n, ez, 0.0, aw * c as f64 * ez, aw * s as f64 * ez, 0.0, 0.0, 0.0, 0.0]
}

// ---- dump files ----------------------------------------------------------------------------

/// Per-pixel record counts in a dump: centre (9), ss2 (4 x 5), ss4 (16 x 5).
pub const CENTRE: usize = 9;
pub const SS2: usize = 20;
pub const SS4: usize = 80;
pub const REC: usize = CENTRE + SS2 + SS4;

pub struct Dump {
    pub w: usize,
    pub h: usize,
    pub hp: f64,
    pub has_ss4: bool,
    pub v: Vec<f64>,
}

impl Dump {
    pub fn rec(&self, p: usize) -> &[f64] {
        &self.v[p * REC..(p + 1) * REC]
    }
    pub fn save(&self, path: &str) -> Result<(), String> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| e.to_string())?);
        let mut hdr = Vec::new();
        hdr.extend_from_slice(b"JDMP0001");
        hdr.extend_from_slice(&(self.w as u32).to_le_bytes());
        hdr.extend_from_slice(&(self.h as u32).to_le_bytes());
        hdr.extend_from_slice(&self.hp.to_le_bytes());
        hdr.extend_from_slice(&(self.has_ss4 as u32).to_le_bytes());
        f.write_all(&hdr).map_err(|e| e.to_string())?;
        for x in &self.v {
            f.write_all(&x.to_le_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn load(path: &str) -> Result<Dump, String> {
        let mut b = Vec::new();
        std::fs::File::open(path)
            .and_then(|mut f| f.read_to_end(&mut b))
            .map_err(|e| format!("{path}: {e}"))?;
        if &b[0..8] != b"JDMP0001" {
            return Err("bad dump magic".into());
        }
        let w = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        let h = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
        let hp = f64::from_le_bytes(b[16..24].try_into().unwrap());
        let has_ss4 = u32::from_le_bytes(b[24..28].try_into().unwrap()) != 0;
        let v = b[28..].chunks(8).map(|c| f64::from_le_bytes(c.try_into().unwrap())).collect();
        Ok(Dump { w, h, hp, has_ss4, v })
    }
}
