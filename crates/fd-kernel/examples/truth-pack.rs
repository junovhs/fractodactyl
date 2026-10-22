//! `cargo run -p fd-kernel --release --example truth-pack -- bench/truth/v1 8`
#[path = "support/mod.rs"]
mod support;
use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), String> {
    let started = Instant::now();
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().unwrap_or("bench/truth/v1".into()));
    let threads: usize = args
        .next()
        .unwrap_or("1".into())
        .parse()
        .map_err(|_| "invalid threads")?;
    if threads == 0 || args.next().is_some() {
        return Err("usage: truth-pack PACK_DIRECTORY [THREADS]".into());
    }
    let edges = support::load_edges(&dir.join("precision-edges.txt"))?;
    let mut paths = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    paths.retain(|p| p.extension().is_some_and(|e| e == "tsv"));
    paths.sort();
    if paths.is_empty() {
        return Err("empty pack".into());
    }
    let mut mismatches = 0;
    let mut known_edges = 0;
    let mut failures = 0;
    let mut points = 0;
    let mut blocks = 0;
    let mut fallbacks = 0;
    for path in &paths {
        let case = support::load(path, threads)?;
        points += case.points.len();
        let last_escape = case
            .points
            .iter()
            .filter(|p| p.class == 'E')
            .map(|p| p.iteration)
            .max()
            .unwrap_or(0);
        for (route, kernel, used, score) in support::run(&case, &edges)? {
            mismatches += score.class_mismatches;
            known_edges += score.known_edges;
            failures += score.scalar_failures;
            blocks += used;
            if route == "bla-unavailable-plain-fallback" {
                fallbacks += 1;
            }
            println!("{{\"case\":\"{}\",\"route\":\"{route}\",\"kernel\":\"{kernel}\",\"points\":{},\"escaped\":{},\"analytic_interior\":{},\"finite_cap_nonescape\":{},\"finite_cap_production_interior\":{},\"latest_oracle_escape\":{last_escape},\"bla_blocks\":{used},\"raw_class_mismatches\":{},\"known_precision_edges\":{},\"unexpected_class_mismatches\":{},\"scalar_failures\":{},\"nu_px\":{},\"de_px\":{},\"normal_px\":{},\"first_failure\":{}}}",case.name,case.points.len(),score.escaped,score.analytic,score.finite_cap,score.finite_cap_interior,score.class_mismatches,score.known_edges,score.class_mismatches-score.known_edges,score.scalar_failures,score.nu_px,score.de_px,score.normal_px,score.first_failure.map_or("null".into(),|i|i.to_string()));
        }
    }
    let elapsed = started.elapsed().as_secs_f64();
    println!("{{\"summary\":true,\"cases\":{},\"points\":{points},\"threads\":{threads},\"seconds\":{elapsed},\"raw_class_mismatches\":{mismatches},\"known_precision_edges\":{known_edges},\"unexpected_class_mismatches\":{},\"scalar_failures\":{failures},\"bla_blocks\":{blocks},\"bla_unavailable_cases\":{fallbacks}}}",paths.len(),mismatches-known_edges);
    if mismatches > known_edges || failures > 0 {
        Err("truth comparison failed".into())
    } else {
        Ok(())
    }
}
