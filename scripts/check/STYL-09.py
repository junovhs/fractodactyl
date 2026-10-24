#!/usr/bin/env python3
"""STYL-09: fd records a last-K stripe-average column that matches direct iteration and
costs little.

At the PROB-22 centre (first camera of docs/2026-10-20/series-frames/results/frames.txt)
at width 1e-12, 640x360 ss 2:
1. `--before` rejects `--columns +stripe` (red: the flag value is unknown there).
2. `fd bench --columns +stripe -o X.fds` writes the stripe column (bit 5).
3. On 1024 fixed escaped samples the column agrees with an independent direct iteration
   (Python integers at 256 fractional bits, exact for these orbits) of the same last-16
   stripe average, k = 5, smooth-interpolated as docs/spec/SAMPLES.md states: mean
   |diff| <= 0.02.
4. Warm render time with the column is at most 1.15x the same bench without it.
5. tools/research/looks/probes/metallic.style renders from X.fds (`fd style render`).
Usage: STYL-09.py --fd FD --before FD --out DIR
"""
import argparse, json, math, struct, subprocess, sys
from pathlib import Path

import numpy as np

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', required=True)
p.add_argument('--before', required=True)
p.add_argument('--out', required=True, type=Path)
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)

frames = root / 'docs/2026-10-20/series-frames/results/frames.txt'
re_, im = next(l.split()[:2] for l in frames.read_text().splitlines()
               if l.strip() and not l.startswith('#'))
WIDTH, K, FREQ = '1e-12', 16, 5.0
view = ['--re', re_, '--im', im, '--width', WIDTH, '--size', '640x360', '--ss', '2']
ok = True


def bench(fd, *extra):
    r = subprocess.run([fd, 'bench', *view, *extra], capture_output=True, text=True)
    return r.returncode, r.stdout, r.stderr


# 1. red before.
code, _, err = bench(a.before, '--columns', '+stripe', '--runs', '1')
print(f'before: exit {code} ({err.strip()[-80:]})')
ok &= code != 0

# 2./4. column written; timing with and without it (warm medians of 3 runs).
fds = a.out / 'stripe.fds'
code, out, err = bench(a.fd, '--columns', '+stripe', '--runs', '3', '-o', str(fds))
if code:
    sys.exit(f'fd bench --columns +stripe failed: {err[-400:]}')
with_s = json.loads(out)['timing']['warm_seconds']
code, out, err = bench(a.fd, '--runs', '3')
if code:
    sys.exit(f'fd bench failed: {err[-400:]}')
without = json.loads(out)['timing']['warm_seconds']
ratio = with_s / without
print(f'warm seconds: {without:.3f} without, {with_s:.3f} with stripe (x{ratio:.3f}, limit 1.15)')
ok &= ratio <= 1.15

buf = fds.read_bytes()
magic, major, minor, cols, nx, ny, ss, _, max_iter, radius, rot = struct.unpack_from(
    '<8sHHIIIIIQdd', buf, 0)
pos = 56
for _ in range(4):
    (n,) = struct.unpack_from('<H', buf, pos)
    pos += 2 + n
align8 = lambda n: (n + 7) & ~7
widths = [1, 8, 4, 2, 4, 4]
offsets, off = {}, align8(pos)
for bit in range(6):
    if cols & (1 << bit):
        offsets[bit] = off
        off += align8(widths[bit] * nx * ny)
print(f'columns 0x{cols:x} minor {minor}')
if 5 not in offsets:
    sys.exit('FAIL: no stripe column')
count = nx * ny
klass = np.frombuffer(buf, np.uint8, count, offsets[0])
stripe = np.frombuffer(buf, '<f4', count, offsets[5])

# 3. 1024 fixed escaped samples against direct iteration.
F = 256
ONE = 1 << F


def fixed(s):
    """Decimal string to an integer at F fractional bits (exact rounding of the decimal)."""
    from fractions import Fraction
    return round(Fraction(s) * ONE)


cre, cim = fixed(re_), fixed(im)
h = fixed(WIDTH) // nx  # sample spacing; exact enough at 256 bits
rng = np.random.default_rng(9)
escaped = np.flatnonzero((klass & 3) == 0)
picks = rng.choice(escaped, 1024, replace=False)
r2 = radius * radius
diffs = []
for idx in picks:
    idx = int(idx)
    i, j = idx % nx, idx // nx
    # c = centre + h (x - i y), x = i + 0.5 - nx/2, y = j + 0.5 - ny/2 (SAMPLES.md).
    cr = cre + (h * (2 * i + 1 - nx)) // 2
    ci = cim - (h * (2 * j + 1 - ny)) // 2
    zr = zi = 0
    terms = []
    for n in range(1, max_iter + 1):
        zr, zi = ((zr * zr - zi * zi) >> F) + cr, ((2 * zr * zi) >> F) + ci
        fr, fi = zr / ONE, zi / ONE
        terms.append(0.5 + 0.5 * math.sin(FREQ * math.atan2(fi, fr)))
        if len(terms) > K + 1:
            terms.pop(0)
        if fr * fr + fi * fi > r2:
            now = sum(terms[-K:]) / len(terms[-K:])
            prev = terms[-K - 1:-1] if len(terms) > 1 else terms
            before = sum(prev) / len(prev)
            frac = min(max(1 - math.log2(math.log(fr * fr + fi * fi) / math.log(r2)), 0), 1)
            diffs.append(abs(before * (1 - frac) + now * frac - stripe[idx]))
            break
diffs = np.array(diffs)
print(f'oracle: {len(diffs)} samples, mean |diff| {diffs.mean():.5f} (limit 0.02), '
      f'p99 {np.percentile(diffs, 99):.4f}')
ok &= len(diffs) == 1024 and diffs.mean() <= 0.02

# 5. the probe look renders from the column.
png = a.out / 'metallic.png'
r = subprocess.run([a.fd, 'style', 'render', root / 'tools/research/looks/probes/metallic.style',
                    fds, '-o', png], capture_output=True, text=True)
print(f'metallic.style: exit {r.returncode} {r.stderr.strip()[-200:]}')
ok &= r.returncode == 0 and png.stat().st_size > 0

print('PASS' if ok else 'FAIL')
sys.exit(0 if ok else 1)
