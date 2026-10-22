//! Native development commands. This binary is not part of release packages.
mod check;
mod report;
use std::path::Path;

fn run() -> Result<i32, String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h")) {
        println!("fd-dev: native development tools (not distributed)\nusage: fd-dev check [--jobs N] | fd-dev bench report RUN_DIR\nReport reads captured BENC01 logs; preserves legacy historical metadata.");
        return Ok(0);
    }
    match args.as_slice() {
        [command, rest @ ..] if command == "check" => check::run(rest),
        [bench, command, dir] if bench == "bench" && command == "report" => {
            let value = report::run(Path::new(dir))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
            );
            Ok(0)
        }
        _ => {
            Err("usage: fd-dev check [--jobs N] | fd-dev bench report RUN_DIR (see --help)".into())
        }
    }
}

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("fd-dev: {error}");
            2
        }
    };
    std::process::exit(code);
}
