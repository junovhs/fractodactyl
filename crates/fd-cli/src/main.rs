//! `fd`: render palette-independent samples once, shade them any number of times.
mod addr;
mod args;
mod bench;
mod chunk;
mod compare;
mod compile;
mod explore;
mod control;
mod lod;
mod manifest;
mod orbit;
mod path;
mod plan;
mod play;
mod render;
mod reuse;
mod schedule;
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
  fd shade IN.fds|DIR [--look umber,palette,relief] -o OUTDIR
  fd shade <umber|palette|relief> in.fds out.png
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
  fd schedule PATH --store DIR [--size WxH] [--ss N] [--iter N] [--columns C]
              [--kernel K] [--fps F] [--lead S] [--workers N]
              [--policy slack|edf|first-use] [--bla frame|group|none] [--slab N]
              [--on-miss fail|report]
  fd control PATH [--size WxH] [--ss N] [--iter N] [--columns C] [--threads N]
             [--kernel K] [--bla none|per-frame] [--runs N] [-o DIR]
             [--oracle tools/oracle.py [--every N] [--k K] [--python python3]]
  fd compare A_DIR B_DIR [--px P] [--frames A..B]
  fd path zoom --re X --im Y --from W0 --to W1 --seconds S --fps F [--rotation R]
  fd compile PATH --store DIR [--size WxH] [--ss N] [--iter N] [--columns C]
             [--kernel K] [--tile-px N] [--bla level|frame|group|none] [--bla-levels K]
             [--fps F] [--lead S] [--workers N] [--policy slack|edf|first-use]
             [--slab N] [--target BYTES] [--cap BYTES] [--on-miss fail|report]
  fd play COMPILE_LOG --store DIR [--frames A..B] [--threads N] [-o DIR]
          [--look L[,L...]] [--mp4 FILE]
          [--oracle tools/oracle.py [--every N] [--k K] [--python python3]]
  fd lod in.fds [--tile-px N] [--max-px E]
  fd explore [--port 8737] [--threads N]   (local browser explorer)
exit: 0 ok, 1 fd schedule (or fd compile --on-miss fail) missed a deadline,
      2 any other error";

/// Exit code of `fd schedule` when a chunk misses its first-use deadline (without
/// `--on-miss report`). Every other failure (usage, build, I/O) exits 2.
pub(crate) const EXIT_MISSED: i32 = 1;

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
        Some("schedule") => schedule::run(&argv[1..]),
        Some("control") => control::run(&argv[1..]),
        Some("compile") => compile::run(&argv[1..]),
        Some("compare") => compare::run(&argv[1..]),
        Some("play") => play::run(&argv[1..]),
        Some("path") => path::run(&argv[1..]),
        Some("lod") => lod::run(&argv[1..]),
        Some("explore") => explore::run(&argv[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = result {
        eprintln!("fd: {e}");
        std::process::exit(2);
    }
}
