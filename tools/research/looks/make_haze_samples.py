#!/usr/bin/env python3
"""Generate two small real .fds fixtures for FX-07 (never rounds deep centres)."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "tools/research/looks/samples"
FD = ROOT / "target/release/fd"
if not FD.is_file():
    FD = FD.with_suffix(".exe")


def render(name, re, im, width):
    out = OUT / f"{name}.fds"
    cmd = [str(FD), "render", "--re", re, "--im", im, "--width", width,
           "--size", "128x72", "--ss", "1", "--iter", "100000",
           "--columns", "nu,de,normal", "--threads", "4", "-o", str(out)]
    print("Rendering", name, "width", width, flush=True)
    subprocess.run(cmd, cwd=ROOT, check=True)
    print(out.relative_to(ROOT), out.stat().st_size, "bytes", flush=True)


def main():
    if not FD.is_file():
        raise SystemExit("First run: cargo build --release")
    OUT.mkdir(parents=True, exist_ok=True)
    # VIDE-01 frame 721, in the 47-48-decade approach to the period-764 minibrot.
    frames = [line.split() for line in (ROOT / "bench/path-atlas-v0.txt").read_text().splitlines()
              if line.strip() and not line.startswith("#")]
    assert len(frames) == 750, "VIDE-01 path changed"
    render("dense", *frames[720])
    # Wide Mandelbrot: an ordinary frame with sparse fine-band fringe only.
    render("ordinary", "-0.75", "0", "3")
    print("Commit these two .fds files, then run haze_check.py", flush=True)


if __name__ == "__main__":
    main()
