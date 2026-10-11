#!/usr/bin/env python3
"""Variety planner prototype: zoom one decade (10x) at a time, each decade as different as
possible from the last, judged by a 4-number spectral signature of a tiny fd render.

  decades.py --out DIR [--re RE --im IM --width W] [--min-decades 20] [--film] [--rate 0.09691] [--size WxH] [--preset master-ice]

Signature of a view (from fd's math columns, never colours):
  fine      where the detail sits: spectral centroid of the distance-estimate field (0 coarse .. 1 fine)
  rough     spectral slope (how evenly detail spreads over scales)
  aniso     directionality: power concentrated in a few directions (streaks, spokes, spirals)
  interior  fraction of the frame inside the set (black)
"""
import argparse
import json
import math
import os
import struct
import subprocess
import tempfile
import time
from pathlib import Path

import mpmath as mp
import numpy as np

FD = os.environ.get('FD', '/home/juno/fractodactyl/target/ishoo-worktrees/shared/release/fd')
TMP = Path(tempfile.mkdtemp(prefix='decades-'))
MODE = os.environ.get('MODE', 'target')  # target: whole-path minibrot planner; vary/follow: older greedy modes


def read_fds(path):
    b = Path(path).read_bytes()
    assert b[:8] == b'FDSAMPLE'
    mask, nx, ny, ss = struct.unpack_from('<IIII', b, 12)
    off = 56
    for _ in range(4):  # re, im, width, kernel strings
        n, = struct.unpack_from('<H', b, off)
        off += 2 + n
    off = (off + 7) // 8 * 8
    cols = {}
    for bit, name, dt in ((0, 'class', np.uint8), (1, 'nu', np.float64), (2, 'de', np.float32),
                          (3, 'normal', np.uint16), (4, 'bound', np.float32)):
        if mask >> bit & 1:
            a = np.frombuffer(b, dt, nx * ny, off).reshape(ny, nx)
            cols[name] = a
            off = (off + a.nbytes + 7) // 8 * 8
    return cols


def render(re, im, width, size=(96, 54), it=50000, name='v'):
    f = TMP / f'{name}.fds'
    t = time.time()
    subprocess.run([FD, 'render', '--re', re, '--im', im, '--width', width, '--size', '%dx%d' % size,
                    '--iter', str(it), '--columns', 'nu,de', '-o', str(f)],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return read_fds(f), time.time() - t


def signature(cols):
    kind = cols['class'] & 3
    esc = kind == 0
    interior = float((kind != 0).mean())
    de = np.where(esc, cols['de'], 1e-3).astype(float)
    field = np.log10(np.clip(de, 1e-3, 1e3))  # structure in pixels: boundary = low
    field -= field.mean()
    ny, nx = field.shape
    win = np.outer(np.hanning(ny), np.hanning(nx))
    P = np.abs(np.fft.fftshift(np.fft.fft2(field * win))) ** 2
    fy, fx = np.meshgrid(np.fft.fftshift(np.fft.fftfreq(ny)), np.fft.fftshift(np.fft.fftfreq(nx)), indexing='ij')
    k = np.hypot(fx, fy) / 0.5  # 0..~1.4, 1 = Nyquist along an axis
    band = (k > 0.03) & (k <= 1)
    p = P[band]
    tot = p.sum() + 1e-30
    fine = float((k[band] * p).sum() / tot)
    bins = np.linspace(0.03, 1, 13)
    rad = np.array([P[(k > a) & (k <= b)].mean() + 1e-30 for a, b in zip(bins[:-1], bins[1:])])
    rough = float(np.polyfit(np.log(bins[1:]), np.log(rad), 1)[0])
    ang = np.arctan2(fy, fx)[band] % np.pi
    hist = np.bincount(np.minimum((ang / np.pi * 12).astype(int), 11), weights=p, minlength=12) / tot
    aniso = float(1 - (-(hist * np.log(hist + 1e-12)).sum()) / math.log(12))  # 0 even .. 1 one direction
    dust = float(P[(k > 0.6) & (k <= 1)].sum() / tot)  # power at the finest scales: grain, specks
    edge = float((esc & (cols['de'] < 1)).mean())  # boundary coverage; ~0 = flat empty colour
    return dict(fine=fine, rough=rough, aniso=aniso, interior=interior, dust=dust, edge=edge)


SCALE = dict(fine=0.15, rough=1.0, aniso=0.15, interior=0.3)  # rough typical spreads, to normalise


def distance(a, b):
    return math.sqrt(sum(((a[x] - b[x]) / SCALE[x]) ** 2 for x in SCALE))


def candidates(cols, n, rng):
    """Spread-out points in the central part of the frame: half on the boundary next to small
    solid blobs (minibrots, where deep zooms get dramatic), half anywhere on the boundary."""
    kind = cols['class'] & 3
    de = cols['de']
    ny, nx = de.shape
    inside = kind != 0
    near = np.zeros_like(inside)
    for dy in (-2, -1, 0, 1, 2):
        for dx in (-2, -1, 0, 1, 2):
            near |= np.roll(np.roll(inside, dy, 0), dx, 1)
    boundary = (kind == 0) & (de < 1.5)
    central = np.zeros_like(inside)
    central[int(0.15 * ny):int(0.85 * ny), int(0.15 * nx):int(0.85 * nx)] = True
    # Hidden minibrots: local peaks of the escape count (they pull nearby orbits in for longer).
    nu = np.where(kind == 0, cols['nu'], 0)
    peak = (kind == 0) & central
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if dy or dx:
                peak &= nu >= np.roll(np.roll(nu, dy, 0), dx, 1)
    out = []
    ys, xs = np.nonzero(peak)
    for i in np.argsort(-nu[ys, xs]):
        if len(out) >= n // 3:
            break
        if all((xs[i] - x) ** 2 + (ys[i] - y) ** 2 > (nx / 12) ** 2 for x, y in out):
            out.append((xs[i], ys[i]))
    for pool, quota in ((boundary & near & central, 2 * n // 3), (boundary & central, n)):
        ys, xs = np.nonzero(pool)
        for i in rng.permutation(len(xs)):
            if len(out) >= quota:
                break
            if all((xs[i] - x) ** 2 + (ys[i] - y) ** 2 > (nx / 12) ** 2 for x, y in out):
                out.append((xs[i], ys[i]))
    return out


def plan(re, im, width, decades, n, seed=1, out=None):
    rng = np.random.default_rng(seed)
    mp.mp.dps = 30
    cre, cim, w = mp.mpf(re), mp.mpf(im), mp.mpf(width)
    cols, _ = render(str(cre), str(cim), mp.nstr(w, 17), name='d0')
    sig = signature(cols)
    steps = [dict(decade=0, re=str(cre), im=str(cim), width=mp.nstr(w, 6), sig=sig)]
    print('decade 0', {k: round(v, 3) for k, v in sig.items()}, flush=True)
    for d in range(1, decades + 1):
        mp.mp.dps = int(30 + max(0, -mp.log10(w)))
        best = None
        ny, nx = cols['de'].shape
        h = w / nx
        for i, (x, y) in enumerate(candidates(cols, n, rng)):
            c_re = cre + h * (x + mp.mpf('0.5') - mp.mpf(nx) / 2)
            c_im = cim - h * (y + mp.mpf('0.5') - mp.mpf(ny) / 2)
            cc, secs = render(mp.nstr(c_re, mp.mp.dps), mp.nstr(c_im, mp.mp.dps), mp.nstr(w / 10, 17), name=f'c{d}_{i}')
            s = signature(cc)
            if MODE == 'follow':
                score = float(np.where((cc['class'] & 3) == 0, cc['nu'], 0).max()) - (1e9 if s['interior'] > 0.6 else 0)
            else:
              score = (distance(s, sig) - 3 * max(0, s['interior'] - 0.35) - 8 * max(0, s['dust'] - 0.25)
                     - 0.5 * secs)
            if best is None or score > best[0]:
                best = (score, c_re, c_im, cc, s, secs)
        if best is None:
            print('decade', d, ': no boundary to follow, stop')
            break
        _, cre, cim, cols, sig, secs = best
        w = w / 10
        steps.append(dict(decade=d, re=mp.nstr(cre, mp.mp.dps), im=mp.nstr(cim, mp.mp.dps), width=mp.nstr(w, 6),
                          sig=sig, score=best[0], secs=secs))
        print('decade', d, {k: round(v, 3) for k, v in sig.items()}, 'jump %.2f' % best[0], flush=True)
    if out:
        Path(out).mkdir(parents=True, exist_ok=True)
        (Path(out) / 'plan.json').write_text(json.dumps(steps, indent=1))
    return steps


def strip(steps, out):
    """One 320x180 frame per decade, centred on the final point (what the film will show)."""
    last = steps[-1]
    imgs = []
    for s in steps:
        f = TMP / f's{s["decade"]}.fds'
        subprocess.run([FD, 'render', '--re', last['re'], '--im', last['im'], '--width', s['width'], '--size', '320x180',
                        '--ss', '2', '--iter', '100000', '--columns', 'nu,de,normal', '-o', str(f)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run([FD, 'shade', str(f), '--preset', 'ice', '-o', str(TMP)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        imgs.append(str(TMP / f's{s["decade"]}.studio.png'))
    subprocess.run(['montage', *imgs, '-tile', '4x', '-geometry', '+3+3', str(Path(out) / 'strip.png')], check=True)




# ---------------------------------------------------------------- v2: whole-path targets

def nucleus(c, w, max_period=4000):
    """Period of the minibrot nearest c (atom-domain: the n where |z_n| hits its smallest value),
    Newton to its nucleus, and its size (mandelbrot-numerics size estimate). None unless Newton
    really converges (|z_p| tiny): a minibrot found from a window of width w can be ~w^2 small,
    so the precision is sized for that, not for the window."""
    digits = int(-mp.log10(w))
    with mp.workdps(40 + 3 * max(0, digits)):
        c = mp.mpc(c)
        z, best, p = mp.mpc(0), mp.inf, 0
        for n in range(1, max_period + 1):
            z = z * z + c
            a = abs(z)
            if a > 4:
                break
            if a < best:
                best, p = a, n
        if p == 0:
            return None
        for _ in range(80):
            z, dz = mp.mpc(0), mp.mpc(0)
            for _ in range(p):
                dz = 2 * z * dz + 1
                z = z * z + c
            if dz == 0:
                return None
            c -= z / dz
            if abs(z) < mp.mpf(10) ** -(20 + 2 * max(0, digits)):
                break
        else:
            return None
        if abs(c - mp.mpc(c)) > w:
            return None
        z, l, b = mp.mpc(0), mp.mpc(1), mp.mpc(1)
        for _ in range(1, p):
            z = z * z + c
            l = 2 * z * l
            b += 1 / l
        size = abs(1 / (b * l * l))
        return (c, p, size) if 0 < size < w else None


def targets(re, im, width, n=24, depth2=6, seed=1):
    """Minibrot targets: nuclei found from boundary points of the start view, then deeper
    nuclei found among the decorations around the first few."""
    rng = np.random.default_rng(seed)
    out = {}

    def harvest(cre, cim, w, tag):
        mp.mp.dps = int(30 + max(0, -mp.log10(w)))
        cols, _ = render(mp.nstr(cre, mp.mp.dps), mp.nstr(cim, mp.mp.dps), mp.nstr(w, 17), size=(160, 90), name='h' + tag)
        ny, nx = cols['de'].shape
        h = w / nx
        found = []
        for x, y in candidates(cols, n, rng):
            c = mp.mpc(cre + h * (x + mp.mpf('0.5') - nx / 2), cim - h * (y + mp.mpf('0.5') - ny / 2))
            r = nucleus(c, w)
            if r and r[2] < w / 100 and abs(r[0] - mp.mpc(cre, cim)) < w:
                key = (r[1], mp.nstr(mp.re(r[0]), 12), mp.nstr(mp.im(r[0]), 12))
                if key not in out:
                    out[key] = r
                    found.append(r)
        return found

    mp.mp.dps = 30
    first = harvest(mp.mpf(re), mp.mpf(im), mp.mpf(width), '0')
    for i, (c, p, size) in enumerate(sorted(first, key=lambda r: -r[2])[:depth2]):
        harvest(mp.re(c), mp.im(c), size * 400, str(i + 1))  # its decorations: embedded Julia sets
    return list(out.values())


def path_score(c, size, w0, rng=None):
    """Tiny render per decade on a straight zoom into c, down to the minibrot filling the frame."""
    decades = int(mp.floor(mp.log10(w0 / (size * 6))))
    sigs, secs = [], 0.0
    for d in range(1, decades + 1):
        w = w0 / mp.mpf(10) ** d
        mp.mp.dps = int(30 + max(0, -mp.log10(w)))
        cols, t = render(mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps), mp.nstr(w, 17), size=(64, 36),
                         name='p')
        secs += t
        sigs.append(signature(cols))
    if len(sigs) < 3 or sigs[-1]['interior'] < 0.03:  # the minibrot must actually be there at the end
        return None
    jumps = [distance(a, b) for a, b in zip(sigs, sigs[1:])]
    bad = sum(3 * max(0, s['interior'] - 0.35) + 8 * max(0, s['dust'] - 0.25) + 20 * max(0, 0.03 - s['edge'])
              for s in sigs) / len(sigs)
    return dict(score=float(np.mean(jumps)) - bad + 0.04 * decades, decades=decades, sigs=sigs, secs=secs,
                jumps=[round(j, 2) for j in jumps])


def plan2(re, im, width, out, n=24, seed=1, min_decades=20):
    t = time.time()
    ts = targets(re, im, width, n=n, seed=seed)
    print('%d minibrot targets in %.1fs' % (len(ts), time.time() - t), flush=True)
    scored = []
    for c, p, size in ts:
        if mp.log10(mp.mpf(width) / (size * 6)) < min_decades:
            continue
        r = path_score(c, size, mp.mpf(width))
        if r:
            scored.append((r['score'], c, p, size, r))
            print('  p=%-5d size=%.1e decades=%2d score=%.2f jumps=%s' % (p, size, r['decades'], r['score'], r['jumps']), flush=True)
    scored.sort(key=lambda x: -x[0])
    best = scored[0]
    _, c, p, size, r = best
    mp.mp.dps = int(30 + max(0, -mp.log10(size)))
    final = dict(re=mp.nstr(mp.re(c), mp.mp.dps), im=mp.nstr(mp.im(c), mp.mp.dps), period=p, size=float(size),
                 to=float(size * 6), decades=r['decades'], score=r['score'], jumps=r['jumps'])
    Path(out).mkdir(parents=True, exist_ok=True)
    (Path(out) / 'plan.json').write_text(json.dumps(dict(best=final, all=[dict(p=x[2], size=float(x[3]), score=x[0])
                                                                            for x in scored]), indent=1))
    steps = [dict(decade=d, re=final['re'], im=final['im'], width=mp.nstr(mp.mpf(width) / mp.mpf(10) ** d, 6))
             for d in range(0, r['decades'] + 1)]
    strip(steps, out)
    print('planned in %.1fs; best p=%d size=%.1e over %d decades' % (time.time() - t, p, size, r['decades']))
    return final


def write_speed(width, target, out, rate):
    """One continuous curve for fd film, with a 15 s smooth departure from rest."""
    if not math.isfinite(rate) or rate <= 0:
        raise ValueError('--rate must be positive and finite')
    start, end = mp.log10(mp.mpf(width)), mp.log10(mp.mpf(target))
    if start <= end:
        raise ValueError('target must be narrower than the starting width')
    # The rest span integrates to 2 * depth * seconds_per_decade.
    # Short paths get a proportionally shorter ramp.
    ramp_depth = min(mp.mpf('15') * mp.mpf(str(rate)) / 2, (start - end) / 2)
    file = Path(out) / 'speed.txt'
    file.write_text('# log10_width seconds_per_decade; rest ramps from zero zoom speed\n'
                    + mp.nstr(start, 20) + ' rest\n'
                    + mp.nstr(start - ramp_depth, 20) + ' %.12g\n' % (1 / rate))
    return file


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--re', default='-0.75')
    ap.add_argument('--im', default='0.1')
    ap.add_argument('--width', default='0.5')
    ap.add_argument('--decades', type=int, default=10)
    ap.add_argument('--cands', type=int, default=24)
    ap.add_argument('--seed', type=int, default=1)
    ap.add_argument('--out', required=True)
    ap.add_argument('--min-decades', type=int, default=20)
    ap.add_argument('--film', action='store_true', help='render the winner with fd film --keyframes 2')
    ap.add_argument('--rate', type=float, default=math.log10(1.25),
                    help='film cruise rate in decades/s (default: 1.25x zoom per second)')
    ap.add_argument('--size', default='1280x720')
    ap.add_argument('--preset', default='master-ice')
    a = ap.parse_args()
    t = time.time()
    if MODE == 'target':
        final = plan2(a.re, a.im, a.width, a.out, a.cands, a.seed, a.min_decades)
        print('final', final['re'], final['im'], 'to', final['to'])
        curve = write_speed(a.width, final['to'], a.out, a.rate)
        print('speed curve:', curve)
        cmd = [FD, 'film', final['re'], final['im'], '--from', a.width, '--to', '%.3e' % final['to'], '--speed', str(curve),
               '--fps', '30', '--size', a.size, '--keyframes', '2', '--preset', a.preset, '--mp4', str(Path(a.out) / 'film.mp4')]
        print('film:', ' '.join(cmd))
        if a.film:
            t = time.time()
            subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)
            print('film rendered in %.0fs' % (time.time() - t))
        return
    steps = plan(a.re, a.im, a.width, a.decades, a.cands, a.seed, a.out)
    print('planned %d decades in %.1fs' % (len(steps) - 1, time.time() - t))
    strip(steps, a.out)
    last = steps[-1]
    print('final', last['re'], last['im'], last['width'])


if __name__ == '__main__':
    main()
