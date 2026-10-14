//! Frozen whole-frame finish inputs, captured once and reused by all arms.
use fd_samples::{Column, ColumnSet, View};
use std::{
    io::{Read, Write},
    path::PathBuf,
};
const WIDTHS: [&str; 6] = ["1e-37", "1e-40", "1e-43", "1e-46", "2e-48", "2e-49"];

fn paths(width: &str, nx: u32, ny: u32) -> Result<(PathBuf, PathBuf), String> {
    if !WIDTHS.contains(&width) || nx == 0 || ny == 0 || u64::from(nx) * u64::from(ny) > 1920 * 1080
    {
        return Err("use a listed depth and <=1080p grid".into());
    }
    let tree = std::process::Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .output()
        .map_err(|e| e.to_string())?;
    let tree = String::from_utf8(tree.stdout).map_err(|e| e.to_string())?;
    let tree = tree.trim();
    if !tree.bytes().all(|b| b.is_ascii_hexdigit()) || tree.len() != 40 {
        return Err("cannot bind input cache to source tree".into());
    }
    let root = std::env::var_os("FD_GPU_ROUTE_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("fd-gpu-route-inputs"));
    if !root.is_absolute() {
        return Err("FD_GPU_ROUTE_CACHE must be absolute and outside checkout".into());
    }
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let top = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| e.to_string())?;
    if !top.status.success() {
        return Err("cannot identify checkout for cache bounds".into());
    }
    let top = String::from_utf8(top.stdout).map_err(|e| e.to_string())?;
    let cwd = std::fs::canonicalize(top.trim()).map_err(|e| e.to_string())?;
    if root.starts_with(cwd) {
        return Err("input cache must stay outside the checkout".into());
    }
    let base = root.join(format!("{tree}-{width}-{nx}x{ny}"));
    Ok((base.with_extension("inputs"), base.with_extension("fds")))
}

pub fn capture(width: &str, nx: u32, ny: u32) -> Result<(), String> {
    let (path, reference_path) = paths(width, nx, ny)?;
    if path.exists() && reference_path.exists() {
        println!(
            "capture_frame=REUSED width={width} grid={nx}x{ny} inputs={}",
            path.display()
        );
        return Ok(());
    }
    let started = std::time::Instant::now();
    let zone = fd_kernel::Zone::parse(include_str!("../../../../bench/zones/v0-core.zone"))?;
    let p = fd_kernel::Params {
        nx,
        ny,
        ss: 1,
        max_iter: 20_000,
        escape_radius: 1e10,
        columns: ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]),
        threads: 1,
        tier: None,
    };
    let view = View {
        center_re: "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502".into(),
        center_im: "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922".into(),
        width: width.into(), rotation: 0.0,
    };
    let (header, samples, states, c) = zone.probe_finish_inputs(&view, &p)?;
    let capture_ms = started.elapsed().as_secs_f64() * 1000.0;
    let count = states.iter().flatten().count();
    let camera = [
        width.parse::<f64>().map_err(|e| e.to_string())? / f64::from(nx),
        1.0,
        0.0,
        c[0],
        c[1],
        1e20,
    ];
    let partial = path.with_extension("inputs.partial");
    let file = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
    let mut out = std::io::BufWriter::new(file);
    out.write_all(b"FDRFIN03").map_err(|e| e.to_string())?;
    out.write_all(&(count as u32).to_le_bytes())
        .map_err(|e| e.to_string())?;
    for value in camera {
        out.write_all(&value.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    for (index, state) in states.iter().enumerate() {
        if let Some(state) = state {
            out.write_all(&(index as u32).to_le_bytes())
                .map_err(|e| e.to_string())?;
            let n = u32::try_from(state.iteration).map_err(|_| "iteration overflow")?;
            out.write_all(&n.to_le_bytes()).map_err(|e| e.to_string())?;
            for value in [
                state.z[0],
                state.z[1],
                state.derivative[0],
                state.derivative[1],
            ] {
                out.write_all(&value.to_le_bytes())
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    let ref_partial = reference_path.with_extension("fds.partial");
    fd_samples::write(
        std::fs::File::create(&ref_partial).map_err(|e| e.to_string())?,
        &header,
        &samples,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(ref_partial, &reference_path).map_err(|e| e.to_string())?;
    std::fs::rename(partial, &path).map_err(|e| e.to_string())?;
    println!(
        "capture_frame=PASS width={width} grid={nx}x{ny} inputs={count} bypassed={} capture_ms={capture_ms:.3} frozen_inputs={} frozen_reference={}",
        nx as usize * ny as usize - count,
        path.display(),
        reference_path.display()
    );
    Ok(())
}

pub fn run(
    width: &str,
    nx: u32,
    ny: u32,
    arm: Option<&str>,
    precision: Option<&str>,
) -> Result<(), String> {
    let (path, reference_path) = paths(width, nx, ny)?;
    let mut reader = fd_samples::Reader::open(&reference_path)
        .map_err(|e| format!("run capture-frame first: {e}"))?;
    if reader.header.nx != nx || reader.header.ny != ny || reader.header.view.width != width {
        return Err("frozen reference camera mismatch".into());
    }
    let reference = reader
        .read(ColumnSet::of(&[Column::Nu, Column::De, Column::Normal]))
        .map_err(|e| e.to_string())?;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > 60 + u64::from(nx) * u64::from(ny) * 40 {
        return Err("oversized frozen input file".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() < 60 || &bytes[..8] != b"FDRFIN03" {
        return Err("invalid frozen input header".into());
    }
    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if count == 0 || count > nx as usize * ny as usize || bytes.len() != 60 + count * 40 {
        return Err("invalid frozen input count".into());
    }
    let camera = bytes[12..60]
        .chunks_exact(8)
        .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    let mut indices = Vec::with_capacity(count);
    let mut iterations = Vec::with_capacity(count);
    let mut input = Vec::with_capacity(count * 4);
    for record in bytes[60..].chunks_exact(40) {
        let index = u32::from_le_bytes(record[..4].try_into().unwrap()) as usize;
        if index >= reference.class.len() || indices.last().is_some_and(|last| *last >= index) {
            return Err("invalid frozen input index".into());
        }
        indices.push(index);
        iterations.push(u32::from_le_bytes(record[4..8].try_into().unwrap()));
        input.extend(
            record[8..]
                .chunks_exact(8)
                .map(|b| f64::from_le_bytes(b.try_into().unwrap())),
        );
    }
    drop(bytes);
    crate::columns::benchmark(
        width,
        nx,
        ny,
        &reference,
        &indices,
        &input,
        &iterations,
        &camera,
        arm,
        precision,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_bounds_reject_oversized_grids_and_unlisted_widths_before_io() {
        assert!(paths("1e-40", 0, 1080).is_err());
        assert!(paths("1e-40", 1921, 1080).is_err());
        assert!(paths("../bad", 16, 9).is_err());
        assert!(paths("1e-1000", 16, 9).is_err());
    }
}
