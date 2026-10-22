#!/usr/bin/env python3
"""BENC-12: time fd and each installed rival on bench/rivals/frames.txt.

Prints a frame x renderer table of cold wall seconds (one process per cell, hard cap
per cell, ">CAP" when killed) and writes OUT/rivals.csv. Rivals are optional: pass
--f3 for fraktaler-3. The column "fd best" is the fastest fd arm on that frame.
"""
import argparse, csv, os, subprocess, sys, time
from decimal import Decimal
from pathlib import Path

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', required=True, type=Path)
p.add_argument('--f3', type=Path, help='fraktaler-3 executable (optional)')
p.add_argument('--out', type=Path, default=root / 'target' / 'rivals-out')
p.add_argument('--size', default='960x540')
p.add_argument('--cap', type=float, default=30.0, help='seconds per cell')
p.add_argument('--threads', type=int, default=os.cpu_count())
p.add_argument('--only', help='comma-separated frame names')
a = p.parse_args()
a.out = a.out.resolve()
a.out.mkdir(parents=True, exist_ok=True)
nx, ny = map(int, a.size.split('x'))

frames = []
for line in (root / 'bench/rivals/frames.txt').read_text().splitlines():
    if line.strip() and not line.startswith('#'):
        name, it, re_, im, w = line.split()
        if not a.only or name in a.only.split(','):
            frames.append((name, int(it), re_, im, w))

def timed(cmd, cwd):
    t = time.perf_counter()
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=a.cap)
    except subprocess.TimeoutExpired:
        return None, 'timeout'
    wall = time.perf_counter() - t
    return (wall, '') if r.returncode == 0 else (None, f'exit {r.returncode}: {r.stderr.strip()[-200:]}')

def fd_arm(label, extra, name, it, re_, im, w):
    d = a.out / 'fd' / name
    d.mkdir(parents=True, exist_ok=True)
    path = d / 'frame.path'
    path.write_text(f'{re_} {im} {w} 0\n')
    return timed([str(a.fd.resolve()), 'control', str(path), '--size', a.size, '--ss', '1',
                  '--iter', str(it), '--columns', 'nu,de,normal', '--threads', str(a.threads),
                  *extra, '-o', str(d / label)], d)

def f3_arm(name, it, re_, im, w):
    d = a.out / 'f3'
    toml = d / f'{name}.f3.toml'
    zoom = f'{4 / (Decimal(w) * ny / nx):.12e}'  # fraktaler-3 zoom: 4 / vertical extent
    toml.write_text('\n'.join([
        f'location.real = "{re_}"', f'location.imag = "{im}"', f'location.zoom = "{zoom}"',
        f'bailout.iterations = {it}', f'bailout.maximum_reference_iterations = {it}',
        f'bailout.maximum_perturb_iterations = {it}', f'bailout.maximum_bla_steps = {it}',
        'bailout.escape_radius = 1e10', f'image.width = {nx}', f'image.height = {ny}',
        'image.subframes = 1', f'render.filename = "{name}"', 'render.save_exr = true',
        'render.exr_channels = ["N0", "N1", "NF", "DEX", "DEY"]', '']))
    return timed([str(a.f3.resolve()), '--batch', '-P', '-w', 'wisdom.toml', toml.name], d)

arms = [('fd plain', lambda *f: fd_arm('plain', ['--bla', 'none'], *f)),
        ('fd bla', lambda *f: fd_arm('bla', ['--bla', 'per-frame'], *f))]
zone = root / 'bench/zones/v0-core.zone'
arms.append(('fd zone', lambda n, *f: fd_arm('zone', ['--bla', 'per-frame', '--zone', str(zone)], n, *f)
             if n.startswith('v0-') else (None, 'n/a')))
if a.f3:
    (a.out / 'f3').mkdir(parents=True, exist_ok=True)
    if not (a.out / 'f3' / 'wisdom.toml').exists():  # fraktaler-3's own hardware config, untimed
        subprocess.run([str(a.f3.resolve()), '-W', '-w', 'wisdom.toml'], cwd=a.out / 'f3',
                       capture_output=True, timeout=120)
    arms.append(('fraktaler-3', f3_arm))

def cell(v, note):
    return f'{v:.2f}' if v is not None else ('n/a' if note == 'n/a' else
                                             f'>{a.cap:g}' if note == 'timeout' else 'FAIL')

names = [n for n, _ in arms]
fd_names = [n for n in names if n.startswith('fd')]
print(f'{a.size} ss1, cap {a.cap:g} s, fd threads {a.threads}', flush=True)
header = ['frame', *names, 'fd best', 'best rival', 'fd vs rival']
print(' | '.join(header), flush=True)
rows = []
for f in frames:
    res = {n: run(*f) for n, run in arms}
    for n, (_, note) in res.items():
        if note not in ('', 'n/a', 'timeout'):
            print(f'  {f[0]} {n}: {note}', file=sys.stderr)
    fd_t = [res[n][0] for n in fd_names if res[n][0] is not None]
    rv_t = [res[n][0] for n in names if not n.startswith('fd') and res[n][0] is not None]
    best_fd = min(fd_t) if fd_t else None
    best_rv = min(rv_t) if rv_t else None
    verdict = ('' if best_rv is None and not [n for n in names if not n.startswith('fd')]
               else 'fd WORSE' if best_rv is not None and (best_fd is None or best_rv < best_fd)
               else 'fd ok')
    row = [f[0], *(cell(*res[n]) for n in names), (f'{best_fd:.2f}' if best_fd is not None else '-'),
           (f'{best_rv:.2f}' if best_rv is not None else '-'), verdict]
    rows.append(row)
    print(' | '.join(row), flush=True)
with open(a.out / 'rivals.csv', 'w', newline='') as fh:
    csv.writer(fh).writerows([header, *rows])
print(f'wrote {a.out / "rivals.csv"}')
