//! Render a crop of a 1920x1080 frame into a jet-AA dump.
//! usage: render REF WIDTH CROPW CROPH OX OY MAX_ITER OUT [ss4]
//!   REF: refgen.py output for the view centre; WIDTH: full-frame width in plane units;
//!   OX, OY: crop origin in output pixels of the 1920x1080 frame.
//! Per output pixel: the centre sample with its third-order jet (binary64), the four ss 2
//! samples of the exact render (binary64, d1 only) and, with `ss4`, the 4x4 truth samples
//! (binary64, re-run in double-double where DE < DD_DE output px).
use jetaa::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const DD_DE: f32 = 0.01;

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 9 {
        return Err("usage: render REF WIDTH CROPW CROPH OX OY MAX_ITER OUT [ss4]".into());
    }
    let r = Ref::load(&a[1])?;
    let width: f64 = a[2].parse().map_err(|_| "width")?;
    let (cw, ch): (usize, usize) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let (ox, oy): (f64, f64) = (a[5].parse().unwrap(), a[6].parse().unwrap());
    let max_iter: u32 = a[7].parse().unwrap();
    let ss4 = a.get(9).map(|s| s == "ss4").unwrap_or(false);
    let hp = width / 1920.0;
    let t0 = std::time::Instant::now();
    let mut v = vec![0.0f64; cw * ch * REC];
    let next = AtomicUsize::new(0);
    let dd_count = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    {
        let rows: Vec<&mut [f64]> = v.chunks_mut(cw * REC).collect();
        let rows: Vec<std::sync::Mutex<&mut [f64]>> = rows.into_iter().map(std::sync::Mutex::new).collect();
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| loop {
                    let py = next.fetch_add(1, Ordering::Relaxed);
                    if py >= ch {
                        break;
                    }
                    let mut row = rows[py].lock().unwrap();
                    let mut ndd = 0;
                    for px in 0..cw {
                        let o = &mut row[px * REC..(px + 1) * REC];
                        // pixel centre, output-pixel offsets from the frame centre
                        let x = ox + px as f64 + 0.5 - 960.0;
                        let y = oy + py as f64 + 0.5 - 540.0;
                        let at = |dx: f64, dy: f64| (hp * (x + dx), -hp * (y + dy));
                        let (cr, ci) = at(0.0, 0.0);
                        o[..CENTRE].copy_from_slice(&sample::<3>(&r, cr, ci, hp, max_iter));
                        for k in 0..4 {
                            let (dx, dy) = ((k % 2) as f64 * 0.5 - 0.25, (k / 2) as f64 * 0.5 - 0.25);
                            let (cr, ci) = at(dx, dy);
                            let s = sample::<1>(&r, cr, ci, hp, max_iter);
                            o[CENTRE + 5 * k..CENTRE + 5 * k + 5].copy_from_slice(&s[..5]);
                        }
                        if ss4 {
                            for k in 0..16 {
                                let (dx, dy) = ((k % 4) as f64 * 0.25 - 0.375, (k / 4) as f64 * 0.25 - 0.375);
                                let (cr, ci) = at(dx, dy);
                                let mut s = sample::<1>(&r, cr, ci, hp, max_iter);
                                if columns(&s).map_or(false, |c| c.1 < DD_DE) {
                                    let xr = x + dx;
                                    let yr = y + dy;
                                    s = sample_dd(&r, Dd::from_prod(hp, xr), Dd::from_prod(-hp, yr), hp, max_iter);
                                    ndd += 1;
                                }
                                let b = CENTRE + SS2 + 5 * k;
                                o[b..b + 5].copy_from_slice(&s[..5]);
                            }
                        }
                    }
                    dd_count.fetch_add(ndd, Ordering::Relaxed);
                });
            }
        });
    }
    let d = Dump { w: cw, h: ch, hp, has_ss4: ss4, v };
    d.save(&a[8])?;
    let unres = (0..cw * ch).filter(|&p| d.rec(p)[0] == 0.0).count();
    let mean_n: f64 = (0..cw * ch).map(|p| d.rec(p)[0]).sum::<f64>() / (cw * ch - unres).max(1) as f64;
    println!(
        "render {} crop={}x{}+{}+{} ref={} max_iter={} centre-unresolved={} mean-escape-iter={:.0} dd-reruns={} secs={:.1}",
        a[8], cw, ch, ox, oy, r.len(), max_iter, unres, mean_n, dd_count.load(Ordering::Relaxed), t0.elapsed().as_secs_f64()
    );
    Ok(())
}
