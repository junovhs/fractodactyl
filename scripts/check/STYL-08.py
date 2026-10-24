#!/usr/bin/env python3
"""STYL-08: dense regions stay smooth instead of speckling, and come closer to an exact render.

Renders PROB-22 film frame 0 (docs/2026-10-20/series-frames/results/frames.txt) at
480x270 with --before twice: ss 2 (the frame under test) and ss 8 (the reference). Each
of tools/research/looks/probes/fire.style and lagoon.style is shaded from the ss 2
samples by both binaries and from the ss 8 samples by --before (64 unfiltered samples per
pixel: the exact-render stand-in). Pixels whose distance estimate is under one ss 2
sample form the dense mask. Per look it reports the mean 3x3 local luminance standard
deviation (speckle) in that mask and the mean per-pixel linear-light error against the
reference, in and outside the mask. Passes when, for both looks, speckle is at most half
of --before's, the dense error is at most 3/4 of --before's, and the error elsewhere
grows by no more than 0.002. Usage: STYL-08.py --fd FD --before FD --out DIR
"""
import argparse, subprocess, sys
from pathlib import Path

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from PIL import Image

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', required=True)
p.add_argument('--before', required=True)
p.add_argument('--out', required=True, type=Path)
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)


def run(*args):
    r = subprocess.run([str(x) for x in args], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f'{args[0]} {args[1]} failed: {r.stderr[-400:]}')


frames = root / 'docs/2026-10-20/series-frames/results/frames.txt'
re_, im, width = next(l.split()[:3] for l in frames.read_text().splitlines()
                      if l.strip() and not l.startswith('#'))
samples = {}
for ss in ['2', '8']:
    samples[ss] = a.out / f'frame0-ss{ss}.fds'
    run(a.before, 'bench', '--re', re_, '--im', im, '--width', width, '--size', '480x270',
        '--ss', ss, '--runs', '1', '-o', samples[ss])

# Brighter than half where de < 0.5 output pixels, i.e. under one sample at ss 2.
mask_graph = a.out / 'mask.style'
mask_graph.write_text('''version 1
surface out
emission none
black = color #000000
white = color #ffffff
zero = scalar 0
one = scalar 1
none = mul black zero
de = source de
dense = smoothstep zero one de
out = mix white black dense
''')
run(a.before, 'style', 'render', mask_graph, samples['2'], '-o', a.out / 'mask.png')
mask = np.asarray(Image.open(a.out / 'mask.png').convert('L')) > 128


def linear(png):
    c = np.asarray(Image.open(png).convert('RGB'), dtype=np.float64) / 255
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def speckle(png):
    lum = np.asarray(Image.open(png).convert('RGB'), dtype=np.float32).mean(-1)
    return sliding_window_view(lum, (3, 3)).std(axis=(-1, -2))[mask[1:-1, 1:-1]].mean()


print(f'dense fraction {mask.mean():.3f}')
ok = mask.mean() > 0.05
for look in ['fire', 'lagoon']:
    graph = root / f'tools/research/looks/probes/{look}.style'
    reference = a.out / f'{look}-reference.png'
    run(a.before, 'style', 'render', graph, samples['8'], '-o', reference)
    truth = linear(reference)
    got = {}
    for name, fd in [('before', a.before), ('after', a.fd)]:
        png = a.out / f'{look}-{name}.png'
        run(fd, 'style', 'render', graph, samples['2'], '-o', png)
        err = np.abs(linear(png) - truth).mean(-1)
        got[name] = speckle(png), err[mask].mean(), err[~mask].mean()
    (sb, db, ob), (sa, da, oa) = got['before'], got['after']
    good = sa <= 0.5 * sb and da <= 0.75 * db and oa <= ob + 0.002
    ok &= good
    print(f'{look}: speckle {sb:.2f} -> {sa:.2f}, dense error {db:.4f} -> {da:.4f}, '
          f'open error {ob:.4f} -> {oa:.4f} {"PASS" if good else "FAIL"}')
print('STYL-08', 'PASS' if ok else 'FAIL')
sys.exit(0 if ok else 1)
