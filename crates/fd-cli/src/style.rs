//! CLI entry point for the opt-in style engine. Existing look routes are untouched.
use crate::args::Args;
use fd_shade::style::Graph;

pub(crate) fn run(argv: &[String]) -> Result<(), String> {
    let a = Args::parse(argv, &["o"])?;
    match a.positional.as_slice() {
        [command, path] if command == "inspect" => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
            let c = Graph::parse(&text)?.compile()?;
            print!("{}", c.report());
            if let Some(out) = a.str("o") {
                std::fs::write(out, &c.body).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Err("usage: fd style inspect GRAPH [-o generated-body.wgsl]".into()),
    }
}
