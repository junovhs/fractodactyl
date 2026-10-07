//! `fd lod`: apply the screen-space error contract (docs/spec/LOD.md) to a `.fds` file
//! and report each tile's state, projected error and accept/refine decision.
use crate::args::Args;
use fd_samples::lod::{tiles, State, FINAL_PX};
use fd_samples::{Column, ColumnSet, Reader};
use std::path::Path;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["tile-px", "max-px"])?;
    let [input] = a.positional.as_slice() else {
        return Err("usage: fd lod in.fds [--tile-px N] [--max-px E]".into());
    };
    let tile_px: u32 = a.num("tile-px", 128)?;
    let max_px: f64 = a.num("max-px", FINAL_PX)?;
    if tile_px == 0 || max_px.is_nan() || max_px < 0.0 {
        return Err("--tile-px must be positive and --max-px non-negative".into());
    }
    let mut r = Reader::open(Path::new(input)).map_err(|e| format!("{input}: {e}"))?;
    // Only the columns the rule reads; absent ones make Bounded samples refine.
    let have = r.header.columns;
    let want = [Column::De, Column::Bound].into_iter().filter(|&c| have.has(c)).fold(ColumnSet::default(), ColumnSet::with);
    let s = r.read(want).map_err(|e| format!("{input}: {e}"))?;
    let report = tiles(&r.header, &s, tile_px, max_px);
    let count = |st| report.iter().filter(|t| t.2.state == st).count();
    let accepted = report.iter().filter(|t| t.2.accept).count();
    println!("tile_px {tile_px}\nmax_px {max_px}\ntiles {}", report.len());
    println!("accepted {accepted}\nrefine {}", report.len() - accepted);
    for st in [State::CertifiedUniform, State::CertifiedApproximate, State::UnresolvedBoundary] {
        println!("state.{} {}", st.name(), count(st));
    }
    for (tx, ty, v) in &report {
        let decision = if v.accept { "accept" } else { "refine" };
        println!("tile {tx} {ty} {} e_px {} {decision} {}", v.state.name(), v.e_px, v.reason);
    }
    Ok(())
}
