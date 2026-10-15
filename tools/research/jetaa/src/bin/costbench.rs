//! Per-sample cost of the exact third-order jet: the same samples with ORDER 1 (z, dz/dc,
//! what films already compute) and ORDER 3 (+ d2, d3), single thread.
//! usage: costbench REF WIDTH MAX_ITER
use jetaa::*;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = Ref::load(&a[1]).unwrap();
    let width: f64 = a[2].parse().unwrap();
    let max_iter: u32 = a[3].parse().unwrap();
    let hp = width / 1920.0;
    let pts: Vec<(f64, f64)> = (0..20000).map(|k| (hp * ((k % 200) as f64 * 2.3 - 230.0), -hp * ((k / 200) as f64 * 2.3 - 115.0))).collect();
    let mut acc = 0.0;
    for round in 0..2 {
        let t = std::time::Instant::now();
        let mut it1 = 0.0;
        for &(x, y) in &pts {
            let s = sample::<1>(&r, x, y, hp, max_iter);
            it1 += s[0];
            acc += s[3];
        }
        let t1 = t.elapsed().as_secs_f64();
        let t = std::time::Instant::now();
        for &(x, y) in &pts {
            let s = sample::<3>(&r, x, y, hp, max_iter);
            acc += s[7];
        }
        let t3 = t.elapsed().as_secs_f64();
        if round == 1 {
            println!("{} samples, {:.0} iterations: order1 {:.2} ns/iter, order3 {:.2} ns/iter, ratio {:.2} ({acc:.1e})", pts.len(), it1, 1e9 * t1 / it1, 1e9 * t3 / it1, t3 / t1);
        }
    }
}
