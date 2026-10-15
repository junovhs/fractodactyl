//! Dump real CPU-zone finish checkpoints (landing z, dz/dc, iteration) for the v0 zone.
//! usage: zone-finish-capture WIDTH NX NY OUT
//! OUT layout (little endian): b"ZFIN0001", u32 nx, u32 ny, f64 h (= width/nx, plane units),
//! f64 cr, f64 ci (shared finish parameter), f64 escape_sq, u32 count, then per captured
//! pixel: u32 index, u32 iteration, f64 zr, zi, dr, di.
use fd_samples::{Column, ColumnSet, View};
use std::io::Write;

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        return Err("usage: zone-finish-capture WIDTH NX NY OUT".into());
    }
    let (width, nx, ny) = (a[1].as_str(), a[2].parse::<u32>().unwrap(), a[3].parse::<u32>().unwrap());
    let zone = fd_kernel::Zone::parse(include_str!("../../../../../../bench/zones/v0-core.zone"))?;
    let escape_radius = 1e10;
    let p = fd_kernel::Params {
        nx,
        ny,
        ss: 1,
        max_iter: 20_000,
        escape_radius,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        threads: 1,
        tier: None,
    };
    let view = View {
        center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
        center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
        width: width.into(),
        rotation: 0.0,
    };
    let t = std::time::Instant::now();
    let (_header, _samples, states, c) = zone.probe_finish_inputs(&view, &p)?;
    let count = states.iter().flatten().count();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&a[4]).map_err(|e| e.to_string())?);
    let h = width.parse::<f64>().map_err(|e| e.to_string())? / f64::from(nx);
    let w = |o: &mut std::io::BufWriter<std::fs::File>, b: &[u8]| o.write_all(b).map_err(|e| e.to_string());
    w(&mut out, b"ZFIN0001")?;
    w(&mut out, &nx.to_le_bytes())?;
    w(&mut out, &ny.to_le_bytes())?;
    for v in [h, c[0], c[1], escape_radius * escape_radius] {
        w(&mut out, &v.to_le_bytes())?;
    }
    w(&mut out, &(count as u32).to_le_bytes())?;
    for (i, s) in states.iter().enumerate() {
        if let Some(s) = s {
            w(&mut out, &(i as u32).to_le_bytes())?;
            w(&mut out, &(s.iteration as u32).to_le_bytes())?;
            for v in [s.z[0], s.z[1], s.derivative[0], s.derivative[1]] {
                w(&mut out, &v.to_le_bytes())?;
            }
        }
    }
    out.flush().map_err(|e| e.to_string())?;
    println!("width={width} grid={nx}x{ny} captured={count} bypassed={} secs={:.1}",
             nx as usize * ny as usize - count, t.elapsed().as_secs_f64());
    Ok(())
}
