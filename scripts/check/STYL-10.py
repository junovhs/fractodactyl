#!/usr/bin/env python3
"""STYL-10: `fd film --style GRAPH` renders a film shaded by a style graph.

Films a 2 s, 320x180 zoom from width 4 to 1e-3 at the PROB-22 centre (first camera of
docs/2026-10-20/series-frames/results/frames.txt) with looks/styles/celestial.style, once
with direct frames (--keyframes off) and once with the default keyframes, saving lossless
frames with --raw-dir. Checks:
1. Both runs exit 0 and write a non-empty mp4.
2. Direct frame 59 matches `fd style render` of a fresh `fd bench` of that frame's view
   (width from the film's progress line) at the same film time, mean |diff| <= 2 levels.
3. Keyframe frame 59 stays within 8 levels mean |diff| of the direct frame (resampling
   from 2x keyframes, not identical) and is not blank.
Usage: STYL-10.py --fd FD --out DIR
"""
import argparse, re, subprocess, sys
from pathlib import Path

import numpy as np
from PIL import Image

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', required=True)
p.add_argument('--out', required=True, type=Path)
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)

frames = root / 'docs/2026-10-20/series-frames/results/frames.txt'
re_, im = next(l.split()[:2] for l in frames.read_text().splitlines()
               if l.strip() and not l.startswith('#'))
style = root / 'looks/styles/celestial.style'
W, H, FPS, F = 320, 180, 60, 59


def run(*args):
    r = subprocess.run([str(x) for x in args], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f'{args[1]} failed ({r.returncode}): {r.stderr[-600:]}')
    return r


def film(name, *extra):
    raw = a.out / f'{name}-raw'
    mp4 = a.out / f'{name}.mp4'
    r = run(a.fd, 'film', re_, im, '--from', '4', '--to', '1e-3', '--seconds', '2',
            '--fps', FPS, '--size', f'{W}x{H}', '--style', style, '--raw-dir', raw,
            '--mp4', mp4, *extra)
    ok = mp4.is_file() and mp4.stat().st_size > 0
    frame = np.fromfile(raw / f'frame-{F:06}.rgb', dtype=np.uint8).reshape(H, W, 3)
    return r, ok, frame.astype(np.float64)


direct_run, direct_ok, direct = film('direct', '--keyframes', 'off')
width = re.search(rf'frame {F + 1}/\d+ width (\S+)', direct_run.stderr)
if not width:
    sys.exit(f'no width for frame {F} in the film log: {direct_run.stderr[-600:]}')
samples = a.out / 'frame59.fds'
run(a.fd, 'bench', '--re', re_, '--im', im, '--width', width.group(1), '--size', f'{W}x{H}',
    '--ss', '2', '--runs', '1', '-o', samples)
reference = a.out / 'frame59-style.png'
run(a.fd, 'style', 'render', style, samples, '-o', reference, '--time', F / FPS,
    '--dither', 'on', '--unresolved', 'interior')
ref = np.asarray(Image.open(reference).convert('RGB'), dtype=np.float64)
keyed_run, keyed_ok, keyed = film('keyframes')

d_ref = np.abs(direct - ref).mean()
d_key = np.abs(keyed - direct).mean()
blank = keyed.std() < 2
ok = direct_ok and keyed_ok and d_ref <= 2 and d_key <= 8 and not blank
print(f'mp4 written: direct {direct_ok}, keyframes {keyed_ok}')
print(f'frame {F} width {width.group(1)}: direct vs fd style render mean |diff| {d_ref:.2f}')
print(f'frame {F}: keyframes vs direct mean |diff| {d_key:.2f}, keyframe std {keyed.std():.1f}')
print('STYL-10', 'PASS' if ok else 'FAIL')
sys.exit(0 if ok else 1)
