#!/usr/bin/env python3
"""PERF-08: deep-minibrot interior is detected instead of iterated to the cap.

1. bench/rivals prob22-2e-29 (960x540, ss1, iter 1e5, --bla per-frame): every sample
   Interior with under 1000 iterations per pixel (the before binary: Unresolved, ~3442).
2. The 24 PROB-22 film frames (480x270, --bla per-frame), against the plain kernel
   (--bla none, same binary) and against --before: no sample moves between Escaped and
   Interior, no Interior sample is lost, and escaped/unresolved flips (BLA rounding at the
   iteration cap) stay under 1e-5 of the samples.
Prints wall seconds for both binaries. Usage: PERF-08.py --fd FD --before FD --out DIR
"""
import argparse, json, re, subprocess, sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fd', required=True)
p.add_argument('--before', required=True)
p.add_argument('--out', required=True, type=Path)
p.add_argument('--threads', default='8')
a = p.parse_args()
a.out.mkdir(parents=True, exist_ok=True)


def control(fd, path, size, out, bla='per-frame'):
    r = subprocess.run([fd, 'control', str(path), '--size', size, '--ss', '1', '--iter', '100000',
                        '--columns', 'nu,de,normal', '--threads', a.threads, '--bla', bla,
                        '-o', str(out)], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f'{fd} control failed: {r.stderr[-400:]}')
    return json.loads(r.stdout.strip().splitlines()[-1])


frame = next(l.split() for l in (root / 'bench/rivals/frames.txt').read_text().splitlines()
             if l.startswith('prob22-2e-29 '))
deep = a.out / 'prob22-2e-29.path'
deep.write_text(f'{frame[2]} {frame[3]} {frame[4]} 0\n')
ok = True
for name, fd in [('before', a.before), ('after', a.fd)]:
    t = control(fd, deep, '960x540', a.out / f'deep-{name}')
    print(f'prob22-2e-29 {name}: {t["timing"]["cold_seconds"]:.2f} s, '
          f'{t["iterations"]["per_pixel"]:.0f} iter/px, classes {t["classes"]}')
if t['classes']['interior'] != 518400 or t['iterations']['per_pixel'] >= 1000:
    print('FAIL: deep minibrot frame not all Interior under 1000 iter/px')
    ok = False

film = a.out / 'film24.path'
film.write_text(''.join(f'{l.split()[0]} {l.split()[1]} {l.split()[2]} 0\n' for l in
                        (root / 'docs/2026-10-20/series-frames/results/frames.txt').read_text().splitlines()
                        if l.strip() and not l.startswith('#')))
for name, fd, bla in [('before', a.before, 'per-frame'), ('after', a.fd, 'per-frame'),
                      ('plain', a.fd, 'none')]:
    t = control(fd, film, '480x270', a.out / f'film-{name}', bla)
    print(f'24 film frames {name}: {t["timing"]["cold_seconds"]:.2f} s, classes {t["classes"]}')


def kinds(x, y):
    r = subprocess.run([a.fd, 'compare', str(a.out / x), str(a.out / y)], capture_output=True, text=True)
    # The totals line can hold non-JSON numbers (inf); read just its `kinds` object.
    return json.loads(re.search(r'"kinds":(\{[^}]*\})', r.stdout.strip().splitlines()[-1])[1])


samples = 24 * 480 * 270
for x, y in [('film-plain', 'film-after'), ('film-before', 'film-after')]:
    k = kinds(x, y)
    print(f'class changes {x} -> {y}', k)
    flips = k['unresolved_to_escaped'] + k['escaped_to_unresolved']
    if k['escaped_to_interior'] or k['interior_to_escaped'] or k['interior_to_unresolved'] \
            or flips > 1e-5 * samples:
        print('FAIL: interior class error or too many escape flips at the cap')
        ok = False
print('PERF-08 PASS' if ok else 'PERF-08 FAIL')
sys.exit(0 if ok else 1)
