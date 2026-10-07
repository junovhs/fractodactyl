//! `fd`: render palette-independent samples once, shade them any number of times.
mod addr;
mod args;
mod bench;
mod chunk;
mod lod;
mod manifest;
mod orbit;
mod plan;
mod render;
mod reuse;
mod shade;

const USAGE: &str = "\
usage:
  fd render --re X --im Y --width W [--size WxH] [--ss N] [--iter N]
            [--columns nu,de,normal,bound] [--threads N] [--rotation R]
            [--kernel auto|f64|fx|scaled] [--store DIR --orbit ID] -o out.fds
            [--store DIR --bla ID]
            [--refine BLOCK_PX [--max-px E]]
  fd bench <render flags> [--runs N] [-o out.fds]
           [--oracle tools/oracle.py [--k K] [--python python3]]
  fd shade <palette|relief> in.fds out.png
  fd info in.fds
  fd addr locate --re X --im Y --level L
  fd addr show <key>
  fd addr sample <key> --grid K --at I,J
  fd addr owner <cell-key> --grid K
  fd chunk put --store DIR --kind K [--encoding N] [--formula F]
               [--precision BITS] [--rounding exact|nearest|outward] <file>
  fd chunk get --store DIR <id> -o out
  fd chunk show --store DIR <id>
  fd chunk verify --store DIR
  fd chunk stats --store DIR
  fd chunk budget --store DIR [--target BYTES] [--cap BYTES]
  fd manifest tile --store DIR --tile KEY [--evidence heuristic,bounded,certified]
                   [--children Q:ID,...] [--refs ID,...]
  fd manifest frame --store DIR --anchor KEY [--offset U,V] [--width W]
                    [--rotation R] [--size WxH] [--ss N] [--iter N]
                    [--columns nu,de,normal] --tiles ID,...
  fd manifest show --store DIR <id>
  fd manifest walk --store DIR [frame-id...]
  fd orbit put --store DIR --re X --im Y --width W [--size WxH] [--ss N]
               [--iter N] [--kernel K] [--slab N]
  fd orbit bla --store DIR <render view flags> [--orbit ID] [--eps E] [--slab N]
  fd plan PATH --size WxH [--ss N] [--tile-px N]
  fd reuse PATH --store DIR [--size WxH] [--ss N] [--iter N] [--columns C]
           [--threads N] [--kernel K] [--slab N]
  fd lod in.fds [--tile-px N] [--max-px E]";

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let result = match argv.first().map(String::as_str) {
        Some("render") => render::run(&argv[1..]),
        Some("bench") => bench::run(&argv[1..]),
        Some("shade") => shade::run(&argv[1..]),
        Some("info") => shade::info(&argv[1..]),
        Some("addr") => addr::run(&argv[1..]),
        Some("chunk") => chunk::run(&argv[1..]),
        Some("manifest") => manifest::run(&argv[1..]),
        Some("orbit") => orbit::run(&argv[1..]),
        Some("plan") => plan::run(&argv[1..]),
        Some("reuse") => reuse::run(&argv[1..]),
        Some("lod") => lod::run(&argv[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = result {
        eprintln!("fd: {e}");
        std::process::exit(2);
    }
}
