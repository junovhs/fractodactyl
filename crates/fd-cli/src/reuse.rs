//! `fd reuse`: multi-frame reference reuse along a known camera path (REF-02,
//! docs/spec/ATLAS.md "Orbit reuse"). Frames at one exact centre share one stored
//! reference orbit, computed once at the deepest frame's precision; every frame is
//! rendered both from that stored orbit and from its own, and the two are compared.
use crate::args::Args;
use crate::orbit::{load, put, same};
use crate::plan::read_path;
use crate::render::{params, FLAGS};
use fd_kernel::{reference, reference_bits, render_stats, render_with};
use fd_samples::{Samples, View};
use std::time::Instant;

const USAGE: &str = "usage: fd reuse PATH --store DIR [render flags without --re/--im/--width] [--slab N]";

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let view_flags = ["re", "im", "width", "rotation", "o"];
    let known: Vec<&str> = FLAGS.iter().copied().filter(|f| !view_flags.contains(f)).chain(["store", "slab"]).collect();
    let a = Args::parse(argv, &known)?;
    let [file] = a.positional.as_slice() else { return Err(USAGE.into()) };
    let p = params(&a)?;
    let size: u32 = a.num("slab", 4096)?;
    let s = crate::chunk::store(&a)?;
    let mut frames: Vec<(View, u32)> = Vec::new();
    for (n, v) in read_path(file)? {
        let bits = reference_bits(&v, &p).map_err(|e| format!("{file}:{n}: {e}"))?;
        frames.push((v, bits));
    }
    // Group frames by exact centre, f64-tier (53-bit) orbits apart from deep ones.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (f, (v, bits)) in frames.iter().enumerate() {
        let joins = |&g: &usize| {
            let (w, b) = &frames[g];
            (*b == 53) == (*bits == 53)
                && same(&v.center_re, &w.center_re).unwrap_or(false)
                && same(&v.center_im, &w.center_im).unwrap_or(false)
        };
        match groups.iter_mut().find(|g| joins(&g[0])) {
            Some(g) => g.push(f),
            None => groups.push(vec![f]),
        }
    }
    // One orbit per group, computed for its deepest frame and stored.
    let mut orbit = vec![(0, 0, String::new(), 0); frames.len()]; // group, lead, manifest id, bits
    let mut shared_s = 0.0;
    for (g, members) in groups.iter().enumerate() {
        let lead = *members.iter().max_by_key(|&&f| (frames[f].1, std::cmp::Reverse(f))).unwrap();
        let t = Instant::now();
        let (r, bits) = reference(&frames[lead].0, &p)?;
        let secs = t.elapsed().as_secs_f64();
        shared_s += secs;
        let id = put(&s, &frames[lead].0, &r, bits, size, false)?.to_string();
        let v = &frames[lead].0;
        println!(
            "orbit {g} centre {} {} precision {bits} points {} frames {} lead {lead} seconds {secs:.6} id {id}",
            v.center_re,
            v.center_im,
            r.len(),
            members.len()
        );
        for &f in members {
            orbit[f] = (g, lead, id.clone(), bits);
        }
    }
    let (mut own_ref, mut own_total, mut reuse_total, mut load_total) = (0.0, 0.0, 0.0, 0.0);
    let (mut reused, mut fallback, mut identical, mut differing, mut broken) = (0, 0, 0, 0u64, 0);
    for (f, (v, need)) in frames.iter().enumerate() {
        let (g, lead, id, bits) = &orbit[f];
        let t = Instant::now();
        let (h0, s0, st0) = render_stats(v, &p)?;
        let own_s = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let loaded = load(&s, id, v)?;
        let load_s = t.elapsed().as_secs_f64();
        let (h1, s1, st1) = render_with(v, &p, Some(loaded))?;
        let reuse_s = t.elapsed().as_secs_f64();
        let d = diff(&s0, &s1);
        let same_kernel = h0.kernel == h1.kernel;
        // Same precision must mean bit-identical output; more precision may differ.
        if bits == need && (d > 0 || !same_kernel) {
            broken += 1;
        }
        // Fallback: no other frame shares the centre, so the frame used its own orbit.
        let role = if groups[*g].len() == 1 {
            fallback += 1;
            "fallback"
        } else if f == *lead {
            "lead"
        } else {
            reused += 1;
            "reused"
        };
        identical += u64::from(d == 0);
        differing += d;
        own_ref += st0.reference_seconds;
        own_total += own_s;
        reuse_total += reuse_s;
        load_total += load_s;
        println!(
            "frame {f} orbit {g} {role} need_bits {need} orbit_bits {bits} own_reference_seconds {:.6} own_seconds {own_s:.6} load_seconds {load_s:.6} reuse_seconds {reuse_s:.6} iterations {} {} differing {d}",
            st0.reference_seconds, st0.iterations, st1.iterations
        );
    }
    let n = frames.len();
    println!("frames {n}\norbits {}\nreused {reused}\nfallback {fallback}", groups.len());
    println!("reuse_ratio {:.3}", n as f64 / groups.len() as f64);
    println!("reference_seconds.per_frame {own_ref:.6}\nreference_seconds.shared {shared_s:.6}\nload_seconds {load_total:.6}");
    println!("reference_seconds.saved {:.6}", own_ref - shared_s - load_total);
    println!("seconds.per_frame {own_total:.6}\nseconds.reuse {:.6}", shared_s + reuse_total);
    println!("identical_frames {identical}\ndiffering_samples {differing}");
    crate::chunk::report_target(&s)?;
    if broken > 0 {
        return Err(format!("{broken} frames rendered differently from an orbit of the same precision"));
    }
    Ok(())
}

/// Samples whose class or any column value differs (compared bit for bit).
fn diff(a: &Samples, b: &Samples) -> u64 {
    fn ne<T: Copy, K: PartialEq>(a: &Option<Vec<T>>, b: &Option<Vec<T>>, i: usize, k: impl Fn(T) -> K) -> bool {
        a.as_ref().map(|v| k(v[i])) != b.as_ref().map(|v| k(v[i]))
    }
    (0..a.class.len())
        .filter(|&i| {
            a.class[i] != b.class[i]
                || ne(&a.nu, &b.nu, i, f64::to_bits)
                || ne(&a.de, &b.de, i, f32::to_bits)
                || ne(&a.normal, &b.normal, i, |x| x)
                || ne(&a.bound, &b.bound, i, f32::to_bits)
        })
        .count() as u64
}
