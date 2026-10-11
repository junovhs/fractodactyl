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
MODE = os.environ.get('MODE', 'directed')  # target: whole-path minibrot planner; vary/follow: older greedy modes


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
    mid = float(P[(k > 0.06) & (k <= 0.32)].sum() / tot)
    edge = float((esc & (cols['de'] < 1)).mean())  # boundary coverage; ~0 = flat empty colour
    reflected = np.fliplr(field)
    symmetry = float(np.clip(1 - np.mean((field - reflected) ** 2) / (2 * np.var(field) + 1e-8), 0, 1))
    return dict(fine=fine, rough=rough, aniso=aniso, interior=interior,
                dust=dust, edge=edge, mid=mid, symmetry=symmetry)


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
    ap.add_argument('--spiral', help='D1-D2: spiral directive for a hero section')
    ap.add_argument('--minutes', type=float, default=10, help='film minutes')
    ap.add_argument('--fps', type=int, default=60, help='film frames per second')
    ap.add_argument('--rate', type=float, default=math.log10(1.25),
                    help='film cruise rate in decades/s (default: 1.25x zoom per second)')
    ap.add_argument('--size', default='3840x2160')
    ap.add_argument('--preset', default='master-ice')
    a = ap.parse_args()
    t = time.time()
    if MODE == 'directed':
        if not (math.isfinite(a.minutes) and a.minutes > 0 and a.fps > 0):
            ap.error('--minutes and --fps must be positive and finite')
        spiral = None
        if a.spiral:
            try:
                spiral = tuple(int(x) for x in a.spiral.split('-'))
                if len(spiral) != 2 or spiral[0] < 1 or spiral[1] <= spiral[0]:
                    raise ValueError()
            except ValueError:
                ap.error('--spiral must be D1-D2 with 1 <= D1 < D2')
        inputs = dict(re=a.re, im=a.im, width=a.width, minutes=a.minutes,
                      spiral=list(spiral) if spiral else None, cands=a.cands,
                      seed=a.seed, min_decades=a.min_decades,
                      size=a.size, fps=a.fps, preset=a.preset)
        saved = Path(a.out) / 'plan.json'
        final = None
        if a.film and saved.is_file() and (Path(a.out) / 'speed.txt').is_file():
            try:
                prior = json.loads(saved.read_text())
                if prior.get('inputs') == inputs:
                    final = prior
                    print('reusing approved plan:', saved, flush=True)
            except (ValueError, OSError):
                pass
        if final is None:
            final = plan_directed(a.re, a.im, a.width, a.out, a.minutes, spiral,
                                  a.cands, a.seed, a.min_decades)
            final['inputs'] = inputs
            saved.write_text(json.dumps(final, indent=2))
        if a.film:
            run_film(final, a.width, a.out, a.fps, a.size, a.preset)
        return
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



# ---------------------------------------------------------------- v3 (PROB-23): directed films

sys_path = str(Path(__file__).resolve().parents[1] / 'shapes')
import sys  # noqa: E402
if sys_path not in sys.path:
    sys.path.insert(0, sys_path)
import geom  # noqa: E402  (PROB-21: Misiurewicz Newton, multiplier, Tan Lei scale)


def spiral_near(c, w):
    """The Misiurewicz point (spiral centre) whose structure c sits in: (q, p) by the closest return
    of the orbit, Newton, then the multiplier, the width its structure fills and its visible turns
    (PROB-21's measures, without the ray rotation). None if Newton does not settle near c."""
    digits = int(-mp.log10(w))
    dps = 40 + 2 * max(0, digits)
    with mp.workdps(dps):
        c = mp.mpc(c)
        # The shallowest spiral centre within half a view of c: the first (q, p), q rising, whose
        # Newton step from c (an estimate of the distance to that root) is under w/2. Spiral
        # centres are dense, so the nearest one is always tiny; the shallowest close one is big.
        # Low-period cycles (1-4) are the strongly twisting ones near bulb edges (v0's family).
        z, d = geom.orbit(c, 304)
        found = next(((q, p) for q in range(2, 300) for p in range(1, 5)
                      if d[q + p] != d[q] and abs(z[q]) < 2 and abs((z[q + p] - z[q]) / (d[q + p] - d[q])) < w), None)
        if found is None:
            return None
        q, p = found
        try:
            m, q, r = geom.misiurewicz(c, q, p, dps, 3 * mp.mpf(w))
            zz, lam, S = geom.local_data(m, q, r, dps)
        except (ValueError, ZeroDivisionError, StopIteration):
            return None
        rq = min(abs(zz[j] - zz[q]) for j in range(1, len(zz)) if j != q and abs(zz[j] - zz[q]) > 0)
        back = mp.fprod(abs(2 * zz[j]) for j in range(1, q))
        width = abs(S) * rq * min(mp.mpf('0.3'), abs(lam) - 1) / back
        levels = math.log(80) / math.log(float(abs(lam)))
        turns = levels * abs(float(mp.arg(lam))) / (2 * math.pi)
        return dict(c=m, q=q, r=r, lam=complex(lam), width=width, turns=turns)


def visible(c, size):
    """The minibrot really is there: solid (or unresolved) pixels in the last frame, 6x its size."""
    with mp.workdps(int(30 + max(0, -mp.log10(size)))):
        cols, _ = render(mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps), mp.nstr(size * 6, 17), size=(64, 36),
                         it=500000, name='vis')
    return float(((cols['class'] & 3) != 0).mean()) >= 0.03


def path_sigs(c, w0, decades):
    sigs = []
    for d in range(1, decades + 1):
        w = w0 / mp.mpf(10) ** d
        with mp.workdps(int(30 + max(0, -mp.log10(w)))):
            cols, _ = render(mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps), mp.nstr(w, 17), size=(64, 36), name='p')
        sigs.append(signature(cols))
    return sigs


def hero_score(s):
    """Cheap mathematical shape score, not a colour/palette judgement (0..1)."""
    coverage = min(1.0, s['edge'] / 0.18)
    silhouette = min(1.0, s['interior'] / 0.12, (0.62 - s['interior']) / 0.30)
    symmetry = max(0.0, s['symmetry'])
    mid = min(1.0, s['mid'] / 0.55)
    haze = max(0.0, (s['dust'] - 0.14) / 0.20) + max(0.0, (s['fine'] - 0.32) / 0.20)
    sparse = max(0.0, (0.045 - s['edge']) / 0.045)
    return float(np.clip(0.43 * coverage + 0.17 * mid + 0.16 * silhouette
                         + 0.14 * symmetry + 0.10 * min(1.0, s['aniso'] / 0.14)
                         - 0.42 * haze - 0.32 * sparse, 0, 1))


def busy_score(s):
    """Coverage with mid-scale variation, discounting empty views and noisy haze."""
    return float(np.clip(0.72 * min(1.0, s['edge'] / 0.18)
                         + 0.28 * min(1.0, s['mid'] / 0.55)
                         - 0.35 * max(0.0, (s['dust'] - 0.15) / 0.2), 0, 1))


def arc_target(t):
    """Calm -> build -> climax at 70% -> quieter minibrot landing."""
    rise = 0.20 + 0.35 * t
    climax = 0.55 * math.exp(-0.5 * ((t - 0.70) / 0.105) ** 2)
    landing = 0.55 * max(0.0, (t - 0.79) / 0.21) ** 1.5
    return float(np.clip(rise + climax - landing, 0.12, 1))


def design_speeds(sigs, lengths, minutes):
    """Smooth seconds/decade; integral, including analytic rest ramp, is exact."""
    if not (math.isfinite(minutes) and minutes > 0):
        raise ValueError('--minutes must be positive and finite')
    hero = np.array([hero_score(s) for s in sigs])
    base = 1 / math.log10(1.25)
    raw = base * (0.82 + 0.58 * hero)
    for _ in range(2):
        raw = np.convolve(np.pad(raw, 1, mode='edge'), [0.2, 0.6, 0.2], 'valid')
    # Avoid a last-minute speed-up: seconds/decade may only rise near the landing.
    for i in range(max(1, len(raw) - 4), len(raw)):
        raw[i] = max(raw[i], raw[i - 1])
    depth = sum(lengths)
    ramp = min(0.96, depth / 4, 15 / (2 * raw[0]))
    for _ in range(8):
        scale = minutes * 60 / (float(np.dot(raw, lengths)) + ramp * raw[0])
        new_ramp = min(0.96, depth / 4, 15 / (2 * raw[0] * scale))
        if abs(new_ramp - ramp) < 1e-10:
            break
        ramp = new_ramp
    scale = minutes * 60 / (float(np.dot(raw, lengths)) + ramp * raw[0])
    pace = raw * scale
    elapsed = pace * lengths
    elapsed[0] += ramp * pace[0]
    return pace.tolist(), elapsed.tolist(), ramp


def arc_samples(sigs, elapsed):
    times = np.cumsum([0.0] + elapsed)
    mid = [(times[i] + times[i + 1]) / (2 * times[-1]) for i in range(len(sigs))]
    target = [arc_target(t) for t in mid]
    achieved = [0.55 * hero_score(s) + 0.45 * busy_score(s) for s in sigs]
    return target, achieved, times.tolist()


def section_vibe(sigs):
    s = {k: float(np.mean([v[k] for v in sigs])) for k in sigs[0]}
    if s['interior'] > 0.32:
        return 'bold bulb silhouettes'
    if s['aniso'] > 0.16:
        return 'radial spokes and spirals'
    if s['edge'] > 0.15:
        return 'ornate filament lace'
    if s['mid'] > 0.45:
        return 'folded mid-scale textures'
    if s['dust'] > 0.22:
        return 'fine haze (low priority)'
    return 'open filaments and negative space'


def design_sections(sigs, elapsed):
    """Partition consecutive decades into 30-60 s coherent runs, with shifts at boundaries."""
    n = len(sigs)
    scaled = np.array([[s[k] / SCALE[k] for k in SCALE] for s in sigs])
    times = np.cumsum([0.0] + elapsed)
    best = [float('inf')] * (n + 1)
    prev = [-1] * (n + 1)
    best[0] = 0.0
    for j in range(1, n + 1):
        for i in range(j):
            duration = times[j] - times[i]
            if n > 1 and (duration < 29.5 or duration > 60.5):
                continue
            v = scaled[i:j]
            coherence = float(np.mean(np.sum((v - v.mean(axis=0)) ** 2, axis=1)))
            jump = float(np.linalg.norm(scaled[i] - scaled[i - 1])) if i else 0
            # Prefer boundaries at actual discontinuities rather than constant flip-flopping.
            cost = (0.40 * coherence + 0.18 * ((duration - 45) / 15) ** 2
                    + 0.48 - 0.23 * min(3.0, jump))
            if best[i] + cost < best[j]:
                best[j], prev[j] = best[i] + cost, i
    if prev[-1] < 0:
        # Very short/long single-decade plans cannot honor 30-60 s; report the exception.
        chunks, start = [], 0
        for j in range(1, n + 1):
            if times[j] - times[start] >= 43 or j == n:
                chunks.append((start, j))
                start = j
    else:
        chunks, j = [], n
        while j:
            i = prev[j]
            chunks.append((i, j))
            j = i
        chunks.reverse()
    sections = []
    for index, (i, j) in enumerate(chunks, 1):
        jump = float(np.linalg.norm(scaled[i] - scaled[i - 1])) if i else 0.0
        sections.append(dict(id=index, first_decade=i + 1, last_decade=j,
                             start_seconds=round(float(times[i]), 3),
                             end_seconds=round(float(times[j]), 3),
                             duration_seconds=round(float(times[j] - times[i]), 3),
                             vibe=section_vibe(sigs[i:j]),
                             transition='opening' if not i else
                             'sudden shift' if jump >= 1.5 else 'gradual drift',
                             shift_distance=round(jump, 3),
                             hero_peak=round(max(hero_score(s) for s in sigs[i:j]), 3)))
    return sections


def assess(sigs, lengths, minutes):
    pace, elapsed, ramp = design_speeds(sigs, lengths, minutes)
    target, achieved, times = arc_samples(sigs, elapsed)
    sections = design_sections(sigs, elapsed)
    climax_values = [achieved[i] for i in range(len(sigs))
                     if 0.58 <= (times[i] + times[i + 1]) / (2 * times[-1]) <= 0.83]
    peak = max(climax_values or achieved)
    fit = float(np.mean((np.array(target) - np.array(achieved)) ** 2))
    ending = float(np.mean([max(0.0, (s['dust'] - 0.15) / 0.20) for s in sigs[-3:]]))
    shifts = sum(s['transition'] == 'sudden shift' for s in sections)
    return dict(score=2.2 * peak - 7.0 * fit - 1.5 * ending + min(shifts, 2) * 0.75,
                pace=pace, elapsed=elapsed, ramp=ramp, target=target, achieved=achieved,
                times=times, sections=sections)


def plan_directed(re, im, width, out, minutes, spiral=None, n=24, seed=1, min_decades=20):
    """Rank verified minibrot paths by whole-film arc, not maximum neighbour-to-neighbour change."""
    t = time.time()
    w0 = mp.mpf(width)
    ideal_depth = minutes * 60 * math.log10(1.25)
    floor = min(min_decades, max(3, int(ideal_depth * 0.70)))
    found = targets(re, im, width, n=n, seed=seed)
    print('%d nucleus candidates in %.1fs' % (len(found), time.time() - t), flush=True)
    best, ranked = None, []
    for c, period, size in found:
        depth = float(mp.log10(w0 / (size * 6)))
        if depth < floor or depth < 3:
            continue
        decades = int(math.floor(depth))
        # A fixed camera can only hold a spiral in the centre if the chosen final
        # minibrot is nearly coincident with its Misiurewicz centre at D2.
        spiral_data = None
        if spiral:
            d1, d2 = spiral
            if not (1 <= d1 < d2 < depth - 1):
                continue
            w2 = w0 / mp.power(10, d2)
            s = spiral_near(c, w0 / mp.power(10, d1))
            if s is None or s['turns'] < 1.5 or abs(c - s['c']) > w2 * mp.mpf('0.06'):
                continue
            spiral_data = dict(re=mp.nstr(mp.re(s['c']), 80),
                               im=mp.nstr(mp.im(s['c']), 80),
                               turns=s['turns'], decades=[d1, d2],
                               max_offset_fraction=float(abs(c - s['c']) / w2))
        if not visible(c, size):
            continue
        sigs = path_sigs(c, w0, decades)
        fractional = depth - decades
        lengths = [1.0] * decades
        if fractional > 1e-4:
            with mp.workdps(int(30 + max(0, -mp.log10(size)))):
                cols, _ = render(mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps),
                                 mp.nstr(size * 6, 17), size=(64, 36), name='end')
            sigs.append(signature(cols))
            lengths.append(fractional)
        result = assess(sigs, lengths, minutes)
        # Aim near the requested 1.25x/s cruise; no fake speed-up to fill short films.
        result['score'] -= 0.8 * abs(math.log(max(depth, 1) / max(ideal_depth, 1)))
        ranked.append(dict(period=period, depth=round(depth, 2), score=round(result['score'], 4)))
        print('  p=%-5d depth=%.1f arc=%.2f peak=%.2f sections=%d' %
              (period, depth, result['score'], max(result['achieved']), len(result['sections'])), flush=True)
        if best is None or result['score'] > best[0]:
            best = (result['score'], c, period, size, sigs, lengths, spiral_data, result)
    if best is None:
        raise SystemExit('no visible minibrot target meeting depth/spiral constraints; try --min-decades 6, --cands 48 or omit --spiral')
    _, c, period, size, sigs, lengths, spiral_data, result = best
    with mp.workdps(int(40 + max(0, -mp.log10(size)))):
        cre, cim, end = (mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps),
                          mp.nstr(size * 6, mp.mp.dps))
    path = Path(out)
    path.mkdir(parents=True, exist_ok=True)
    speed_file = write_directed_speed(width, end, result['pace'], result['ramp'], path)
    rows = []
    for i, (s, length) in enumerate(zip(sigs, lengths)):
        sec = next(x['id'] for x in result['sections'] if x['first_decade'] <= i + 1 <= x['last_decade'])
        rows.append(dict(decade=i + 1, depth_start=i, depth_end=round(i + length, 5),
                         width=end if i == len(sigs) - 1 else mp.nstr(w0 / mp.power(10, i + 1), 17),
                         signature=s, hero=round(hero_score(s), 4), busyness=round(busy_score(s), 4),
                         target=round(result['target'][i], 4),
                         achieved=round(result['achieved'][i], 4),
                         section=sec, seconds=round(result['elapsed'][i], 3),
                         start_seconds=round(result['times'][i], 3),
                         end_seconds=round(result['times'][i + 1], 3)))
    final = dict(re=cre, im=cim, to=end, period=period, size=mp.nstr(size, 17),
                 depth=sum(lengths), minutes=minutes, fps=60, dimensions='3840x2160',
                 score=result['score'], ideal_depth=ideal_depth, spiral=spiral_data,
                 sections=result['sections'], decades=rows,
                 arc=dict(target=result['target'], achieved=result['achieved']),
                 speed_file=str(speed_file), candidates=ranked,
                 caveat='Fixed-centre straight zoom; sections are detected, not guaranteed. Inspect contact sheet.')
    (path / 'plan.json').write_text(json.dumps(final, indent=2))
    draw_arc(result['target'], result['achieved'], path)
    make_contact(rows, cre, cim, path)
    print('planned in %.1fs: p=%d, %.1f decades, %d sections, %d sudden shifts, average %.2f s/decade' %
          (time.time() - t, period, sum(lengths), len(result['sections']),
           sum(s['transition'] == 'sudden shift' for s in result['sections']),
           minutes * 60 / sum(lengths)), flush=True)
    return final


def write_directed_speed(width, end, pace, ramp, out):
    """FILM-12's continuous speed knots; never round deep widths through f64."""
    log_start = mp.log10(mp.mpf(width))
    log_end = mp.log10(mp.mpf(end))
    if log_start <= log_end:
        raise ValueError('target is not narrower than start')
    depth = float(log_start - log_end)
    if not (0 < ramp < min(1, depth)):
        raise ValueError('invalid speed ramp')
    lines = ['# log10_width seconds_per_decade; FILM-12 analytic rest ramp',
             '%.13f rest' % float(log_start),
             '%.13f %.12f' % (float(log_start - mp.mpf(str(ramp))), pace[0])]
    for d in range(1, len(pace)):
        if d >= depth - 1e-8:
            break
        lines.append('%.13f %.12f' % (float(log_start - d), pace[d]))
    f = out / 'speed.txt'
    f.write_text('\n'.join(lines) + '\n')
    return f


def draw_arc(target, achieved, out):
    """Plot via ImageMagick (already required for the contact sheet)."""
    w, h = 1100, 450
    def poly(vals):
        return ' '.join('%.1f,%.1f' % (65 + (w - 95) * i / max(1, len(vals) - 1),
                                      370 - 310 * v) for i, v in enumerate(vals))
    cmd = ['convert', '-size', '%dx%d' % (w, h), 'xc:white',
           '-stroke', '#ddd', '-strokewidth', '1', '-draw', 'line 65,60 65,370 line 65,370 1070,370',
           '-stroke', '#b5b5b5', '-draw', 'line 65,215 1070,215',
           '-stroke', '#2c62b0', '-strokewidth', '3', '-fill', 'none',
           '-draw', 'polyline ' + poly(target),
           '-stroke', '#c65b27', '-draw', 'polyline ' + poly(achieved),
           '-fill', '#333', '-stroke', 'none', '-pointsize', '19',
           '-annotate', '+65+33', 'PROB-23: target (blue) / achieved (orange)',
           '-pointsize', '14', '-annotate', '+65+411', 'Decades / film time (calm opening -> 70% climax -> quiet landing)',
           str(out / 'arc.png')]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)


def make_contact(rows, re, im, out):
    """One fixed-centre, labelled 160x90 thumbnail for every decade."""
    imgs = []
    for row in rows:
        d = row['decade']
        f = TMP / ('arc-%03d.fds' % d)
        subprocess.run([FD, 'render', '--re', re, '--im', im, '--width', row['width'],
                        '--size', '160x90', '--iter', '100000', '--columns', 'nu,de,normal',
                        '-o', str(f)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run([FD, 'shade', str(f), '--preset', 'master-ice', '-o', str(TMP)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        imgs.append((TMP / ('arc-%03d.studio.png' % d), row))
    cmd = ['montage', '-font', 'DejaVu-Sans', '-pointsize', '13', '-background', '#171b24',
           '-fill', 'white', '-tile', '5x', '-geometry', '160x90+5+5']
    for img, row in imgs:
        cmd += ['-label', 'D%02d  hero %.2f  S%d' %
                (row['decade'], row['hero'], row['section']), str(img)]
    subprocess.run(cmd + [str(out / 'contact.jpg')], check=True, stdout=subprocess.DEVNULL)


def run_film(final, width, out, fps, size, preset):
    cmd = [FD, 'film', final['re'], final['im'], '--from', width, '--to', final['to'],
           '--speed', str(Path(out) / 'speed.txt'), '--fps', str(fps), '--size', size,
           '--keyframes', '2', '--gpu', 'on', '--preset', preset,
           '--mp4', str(Path(out) / 'film.mp4')]
    print('film command:', ' '.join(cmd), flush=True)
    start = time.time()
    r = subprocess.run(cmd, check=True, stdout=subprocess.PIPE, text=True)
    elapsed = time.time() - start
    try:
        report = json.loads(r.stdout)
        frames = report.get('frames', round(final['minutes'] * 60 * fps))
        wall = report.get('wall_seconds', elapsed)
    except (ValueError, TypeError):
        frames, wall = round(final['minutes'] * 60 * fps), elapsed
    print('film: %s, %d frames, %.1fs wall, %.3f fps' %
          (Path(out) / 'film.mp4', frames, wall, frames / wall), flush=True)


if __name__ == '__main__':
    main()
