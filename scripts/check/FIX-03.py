#!/usr/bin/env python3
"""FIX-03: BLA on the scaled tier (beyond 2^-900).

bench/rivals deepmini-5e-433 at 160x90, ss1, iter 1e6: `fd control --bla none` against
`--bla per-frame`. Passes when the per-frame record uses a non-empty table with blocks
applied, it is at least 3x faster, and `fd compare` finds no sample moving between
Escaped and Interior and an nu shift under 0.01 px. Usage: FIX-03.py --fd FD --out DIR
"""
import argparse, json, re, subprocess, sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', required=True)
p.add_argument('--out', required=True, type=Path)
p.add_argument('--threads', default='8')
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)
frame = next(l.split() for l in (root / 'bench/rivals/frames.txt').read_text().splitlines()
             if l.startswith('deepmini-5e-433 '))
path = a.out / 'deepmini-5e-433.path'
path.write_text(f'{frame[2]} {frame[3]} {frame[4]} 0\n')
runs = {}
for bla in ['none', 'per-frame']:
    r = subprocess.run([a.fd, 'control', str(path), '--size', '160x90', '--ss', '1', '--iter', frame[1],
                        '--columns', 'nu,de,normal', '--threads', a.threads, '--bla', bla,
                        '-o', str(a.out / bla)], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f'fd control --bla {bla} failed: {r.stderr[-400:]}')
    runs[bla] = json.loads(r.stdout.strip().splitlines()[0])
    t = runs[bla]
    print(f'--bla {bla}: {t["timing"]["cold_seconds"]:.2f} s, kernel {t["kernel"]}, classes {t["classes"]}, '
          f'bla {({k: t["bla"][k] for k in ("use", "dc_max", "dc_exp", "levels", "blocks")} if t.get("bla") else None)}')
c = subprocess.run([a.fd, 'compare', str(a.out / 'none'), str(a.out / 'per-frame')], capture_output=True, text=True)
last = c.stdout.strip().splitlines()[-1]
# The totals line can hold non-JSON numbers (inf); read the fields needed.
kinds = json.loads(re.search(r'"kinds":(\{[^}]*\})', last)[1])
nu_px = float(re.search(r'"nu":\{[^}]*"px_max":([^,}]+)', last)[1])
print('class changes', kinds, 'nu px_max', nu_px)
speedup = runs['none']['timing']['cold_seconds'] / runs['per-frame']['timing']['cold_seconds']
b = runs['per-frame'].get('bla') or {}
ok = (b.get('use') == 'used' and b.get('blocks', 0) > 0 and speedup >= 3.0
      and not (kinds['escaped_to_interior'] or kinds['interior_to_escaped']) and nu_px < 0.01)
print(f'speedup {speedup:.1f}x', 'FIX-03 PASS' if ok else 'FIX-03 FAIL')
sys.exit(0 if ok else 1)
