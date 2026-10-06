//! `fd`: render palette-independent samples once, shade them any number of times.
mod args;
mod render;
mod shade;

const USAGE: &str = "\
usage:
  fd render --re X --im Y --width W [--size WxH] [--ss N] [--iter N]
            [--columns nu,de,normal] [--threads N] -o out.fds
  fd shade <palette|relief> in.fds out.png
  fd info in.fds";

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let result = match argv.first().map(String::as_str) {
        Some("render") => render::run(&argv[1..]),
        Some("shade") => shade::run(&argv[1..]),
        Some("info") => shade::info(&argv[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = result {
        eprintln!("fd: {e}");
        std::process::exit(2);
    }
}
