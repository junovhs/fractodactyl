//! Native development commands. This binary is not part of release packages.
mod report;
use std::path::Path;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        println!("fd-dev: native development tools (not distributed)\nusage: fd-dev bench report RUN_DIR\nReport reads captured BENC01 logs; preserves legacy historical metadata.");
        return Ok(());
    }
    match args.as_slice() {
        [bench, command, dir] if bench == "bench" && command == "report" => {
            let value = report::run(Path::new(dir))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
            );
            Ok(())
        }
        _ => Err("usage: fd-dev bench report RUN_DIR (see --help)".into()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("fd-dev: {error}");
        std::process::exit(2);
    }
}
