#!/usr/bin/env python3
"""Explore deep fixed-centre Mandelbrot films with coherent 30-60 s story sections.

Plan: decades.py --out DIR --minutes 10 --candidates 6 --size 3840x2160 --fps 60
Preview: decades.py --out DIR --minutes 10 --film --pick N --preview-seconds 60
Film: decades.py --out DIR --minutes 10 --film --pick N

Scores come from fd's mathematical samples, not the film palette. Inspect the
ranked contact sheets and arc plots before selecting a candidate for rendering.
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


def render(re, im, width, size=(96, 54), it=50000, name='v', ss=1, columns='nu,de'):
    f = TMP / f'{name}.fds'
    t = time.time()
    subprocess.run([FD, 'render', '--re', re, '--im', im, '--width', width, '--size', '%dx%d' % size,
                    '--iter', str(it), '--ss', str(ss), '--columns', columns, '-o', str(f)],
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
    aniso = float(1 - (-(hist * np.log(hist + 1e-12)).sum()) / math.log(12)) if tot > 1e-20 else 0.0
    dust = float(P[(k > 0.35) & (k <= 1)].sum() / tot)
    mid = float(P[(k > 0.035) & (k <= 0.26)].sum() / tot)
    edge = float((esc & (cols['de'] < 1)).mean())
    blurred = sum(np.roll(np.roll(field, dy, 0), dx, 1)
                  for dy in (-2, -1, 0, 1, 2) for dx in (-2, -1, 0, 1, 2)) / 25
    central = blurred[ny // 4:3 * ny // 4, nx // 4:3 * nx // 4]
    focus = float(np.clip(np.var(central) / (np.var(blurred) + 1e-8), 0, 1))
    # Low-pass activity in 8x8 tiles is harder for single-pixel speckle to fake.
    blocks = [blurred[y:y + 8, x:x + 8] for y in range(0, ny - 7, 8)
              for x in range(0, nx - 7, 8)]
    occupied = float(np.mean([np.std(b) > 0.15 for b in blocks])) if blocks else 0.0
    reflected = np.fliplr(blurred)
    symmetry = float(np.clip(1 - np.mean((blurred - reflected) ** 2) /
                             (2 * np.var(blurred) + 1e-8), 0, 1))
    kind = cols['class'] & 3
    unresolved = float((kind == 2).mean())
    solid = float((kind == 1).mean())
    return dict(fine=fine, rough=rough, aniso=aniso, interior=interior,
                dust=dust, edge=edge, mid=mid, focus=focus,
                occupied=occupied, unresolved=unresolved, solid=solid,
                symmetry=symmetry)


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
    w = mp_scalar(w)
    digits = int(-mp.log10(w))
    with mp.workdps(max(85, 45 + 2 * max(0, digits))):
        c = mp_complex(c)
        seed = c
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
        if abs(c - seed) > w:
            return None
        z, l, b = mp.mpc(0), mp.mpc(1), mp.mpc(1)
        for _ in range(1, p):
            z = z * z + c
            l = 2 * z * l
            if l == 0:
                return None
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
    ap.add_argument('--candidates', type=int, default=6, help='ranked contact sheets to write')
    ap.add_argument('--pick', type=int, help='render this existing cand-N plan; no replan')
    ap.add_argument('--beam', type=int, default=4, help='nucleus parents per starting region')
    ap.add_argument('--rounds', type=int, default=3, help='deepening generations')
    ap.add_argument('--regions', choices=['all', 'single'], default='all')
    ap.add_argument('--preview-seconds', type=float, help='render this many seconds of selected 10-minute path')
    ap.add_argument('--preview-at', type=float, default=0.65, help='preview starts at this film fraction')
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
        if not (math.isfinite(a.minutes) and a.minutes > 0 and a.fps > 0 and
                a.cands > 0 and a.candidates > 0 and a.beam > 0 and a.rounds >= 0):
            ap.error('minutes/fps/cands/candidates/beam must be positive; rounds >= 0')
        if a.preview_seconds is not None and (not math.isfinite(a.preview_seconds) or a.preview_seconds <= 0):
            ap.error('--preview-seconds must be finite and positive')
        if not (math.isfinite(a.preview_at) and 0 <= a.preview_at <= 1):
            ap.error('--preview-at must be in 0..1')
        spiral = None
        if a.spiral:
            try:
                spiral = tuple(int(x) for x in a.spiral.split('-'))
                if len(spiral) != 2 or spiral[0] < 1 or spiral[1] <= spiral[0]:
                    raise ValueError()
            except ValueError:
                ap.error('--spiral must be D1-D2 with 1 <= D1 < D2')
        settings = dict(minutes=a.minutes, fps=a.fps, size=a.size, preset=a.preset)
        if a.film:
            if not a.pick or a.pick < 1:
                ap.error('--film requires --pick N of an existing candidate (no implicit render)')
            chosen = Path(a.out) / ('cand-%d' % a.pick)
            try:
                final = json.loads((chosen / 'plan.json').read_text())
                if final.get('inputs') != settings or not (chosen / 'speed.txt').is_file():
                    ap.error('chosen plan options changed or speed.txt missing; rerun planning')
            except (OSError, ValueError) as exc:
                ap.error('candidate plan not found; run --candidates planning first: %s' % exc)
            run_film(final, chosen, a.fps, a.size, a.preset,
                     a.preview_seconds, a.preview_at)
        else:
            if a.pick or a.preview_seconds:
                ap.error('--pick and --preview-seconds require --film')
            plan_directed(a.re, a.im, a.width, a.out, a.minutes, spiral,
                          a.cands, a.seed, a.min_decades, a.candidates,
                          a.beam, a.rounds, a.regions == 'all', settings)
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
    w = mp_scalar(w)
    digits = int(-mp.log10(w))
    dps = 40 + 2 * max(0, digits)
    with mp.workdps(dps):
        c = mp_complex(c)
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
    size, c = mp_scalar(size), mp_complex(c)
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
    """Mid-scale coherent structure, not a bright field of micro-grain (0..1)."""
    mid = min(1.0, s['mid'] / 0.62)
    occupancy = s.get('occupied', min(1.0, s.get('edge', 0) / 0.18))
    focus = s.get('focus', 0.5)
    silhouette = np.clip(min(s['interior'] / 0.14, (0.70 - s['interior']) / 0.35), 0, 1)
    haze = max(0.0, (s['dust'] - 0.27) / 0.20)
    unresolved = s.get('unresolved', 0.0)
    # Contrast in the middle half and tile occupancy discourage corner-only objects.
    return float(np.clip(0.27 * mid + 0.24 * occupancy + 0.19 * focus
                         + 0.16 * silhouette + 0.14 * s['symmetry']
                         - 0.80 * haze - 0.45 * unresolved, 0, 1))


def busy_score(s):
    """Recognisable mid-scale content; dust is a penalty, never a proxy for detail."""
    return float(np.clip(0.43 * min(1.0, s['mid'] / 0.62)
                         + 0.37 * s.get('occupied', min(1.0, s.get('edge', 0) / 0.18))
                         + 0.20 * s.get('focus', 0.5)
                         - 0.68 * max(0, (s['dust'] - 0.27) / 0.20), 0, 1))


def arc_target(t):
    """Calm -> build -> climax at 70% -> quieter minibrot landing."""
    rise = 0.20 + 0.35 * t
    climax = 0.55 * math.exp(-0.5 * ((t - 0.70) / 0.105) ** 2)
    landing = 0.55 * max(0.0, (t - 0.79) / 0.21) ** 1.5
    return float(np.clip(rise + climax - landing, 0.12, 1))


def design_speeds(sigs, lengths, minutes):
    """A ~10 s/decade cruise at 58 decades, hero lingering and a non-accelerating tail."""
    if not (math.isfinite(minutes) and minutes > 0):
        raise ValueError('--minutes must be positive and finite')
    hero = np.array([hero_score(s) for s in sigs])
    raw = (0.84 + 0.54 * hero) / math.log10(1.25)
    for _ in range(3):
        raw = np.convolve(np.pad(raw, 1, mode='edge'), [0.2, 0.6, 0.2], 'valid')
    for i in range(max(1, len(raw) - 5), len(raw)):
        raw[i] = max(raw[i], raw[i - 1])
    depth = sum(lengths)
    ramp = min(0.95, depth / 4, 15 / (2 * raw[0]))
    for _ in range(12):
        scale = minutes * 60 / (float(np.dot(raw, lengths)) + ramp * raw[0])
        new_ramp = min(0.95, depth / 4, 15 / (2 * raw[0] * scale))
        if abs(new_ramp - ramp) < 1e-12:
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
    if s['dust'] > 0.34:
        return 'dense fine haze (reject/inspect)'
    if s['interior'] > 0.30 and s['mid'] > 0.35:
        return 'large carved bulbs'
    if s['symmetry'] > 0.60 and s['aniso'] > 0.12:
        return 'starburst symmetry'
    if s['aniso'] > 0.16:
        return 'spiral spokes'
    if s['focus'] > 0.55 and s['mid'] > 0.52:
        return 'central sculpted folds'
    if s['occupied'] > 0.58:
        return 'decorated filament forest'
    return 'open branching filaments'


def design_sections(sigs, elapsed):
    """Dynamic programming: 3-6 consecutive similar decades per 30-60 s section."""
    n = len(sigs)
    X = np.array([[s[k] / SCALE[k] for k in SCALE] for s in sigs])
    hero = np.array([hero_score(s) for s in sigs])
    times = np.cumsum([0.0] + elapsed)
    best = [float('inf')] * (n + 1)
    prev = [-1] * (n + 1)
    best[0] = 0.0
    for j in range(3, n + 1):
        for i in range(max(0, j - 6), j - 2):
            duration = times[j] - times[i]
            if not (30 <= duration <= 60):
                continue
            v = X[i:j]
            cohesion = float(np.mean(np.sum((v - v.mean(axis=0)) ** 2, axis=1)))
            jump = float(np.linalg.norm(X[i] - X[i - 1])) if i else 0.0
            jitter = float(np.var(hero[i:j]))
            cost = 0.48 * cohesion + 0.80 * jitter + 0.13 * abs(duration - 45) / 15
            cost += 0.38 - 0.14 * min(2.8, jump)
            if best[i] + cost < best[j]:
                best[j], prev[j] = best[i] + cost, i
    if prev[n] < 0:
        raise ValueError('cannot partition this path into 3-6 decade / 30-60 s sections')
    chunks, j = [], n
    while j:
        i = prev[j]
        chunks.append((i, j))
        j = i
    chunks.reverse()
    sections = []
    for index, (i, j) in enumerate(chunks, 1):
        jump = float(np.linalg.norm(X[i] - X[i - 1])) if i else 0.0
        sections.append(dict(id=index, first_decade=i + 1, last_decade=j,
                             start_seconds=round(float(times[i]), 3),
                             end_seconds=round(float(times[j]), 3),
                             duration_seconds=round(float(times[j] - times[i]), 3),
                             vibe=section_vibe(sigs[i:j]),
                             transition='opening' if not i else
                             'sudden shift' if jump >= 1.5 else 'gradual drift',
                             shift_distance=round(jump, 3),
                             hero_peak=round(float(max(hero[i:j])), 3)))
    return sections


def assess(sigs, lengths, minutes):
    pace, elapsed, ramp = design_speeds(sigs, lengths, minutes)
    target, achieved, times = arc_samples(sigs, elapsed)
    sections = design_sections(sigs, elapsed)
    t = (np.array(times[:-1]) + np.array(times[1:])) / (2 * times[-1])
    values = np.array(achieved)
    hero = np.array([hero_score(s) for s in sigs])
    haze = np.array([max(0.0, (s['dust'] - 0.27) / 0.20) for s in sigs])
    middle = (t >= 0.60) & (t <= 0.80)
    opening = t <= 0.12
    mid_hero = float(np.mean(hero[middle])) if middle.any() else 0
    early_hero = float(np.mean(hero[opening])) if opening.any() else 0
    weight = 1 + 2.5 * middle
    fit = float(np.average((np.array(target) - values) ** 2, weights=weight))
    # Dust cannot be redeemed by one pretty frame. Score sustained hero content,
    # especially at the climax, and strongly discourage intra-section flip-flops.
    bad = float(np.mean(haze))
    filler = float(np.mean(hero < 0.25))
    twitch = float(np.mean(np.abs(np.diff(values)))) if len(values) > 1 else 0
    ending = float(np.mean(haze[-3:]))
    shift = any(sec['transition'] == 'sudden shift' for sec in sections)
    score = (-12 * fit - 4.5 * bad - 2.3 * filler - 2.5 * twitch - 1.8 * ending
             + 4.0 * mid_hero - 1.0 * early_hero + (0.20 if shift else 0))
    passes = bool(mid_hero >= 0.46 and filler <= 0.38 and bad <= 0.28
                  and early_hero < mid_hero and shift)
    return dict(score=score, pace=pace, elapsed=elapsed, ramp=ramp,
                target=target, achieved=achieved, times=times, sections=sections,
                mid_hero=mid_hero, filler_fraction=filler, haze=bad,
                passes_gates=passes, arc_rmse=math.sqrt(fit))


def coarse_score(item):
    """Twelve probes triage before expensive whole-decade and contact-sheet passes."""
    w0 = mp_scalar(item['origin'][3])
    depth = item['depth']
    scores, dust = [], []
    with mp.workdps(max(90, int(55 + depth))):
        c = item['nucleus']
        for fraction in np.linspace(0.03, 0.98, 12):
            d = depth * float(fraction)
            w = w0 / mp.power(10, mp.mpf(str(d)))
            cols, _ = render(mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps),
                             mp.nstr(w, 22), size=(112, 63), it=100000, name='coarse')
            sig = signature(cols)
            target = arc_target(float(fraction))
            achieved = 0.55 * hero_score(sig) + 0.45 * busy_score(sig)
            weight = 3.5 if 0.60 <= fraction <= 0.80 else 1.0
            scores.append(weight * (target - achieved) ** 2)
            dust.append(max(0, (sig['dust'] - 0.27) / 0.2))
    return -10 * float(np.mean(scores)) - 4.0 * float(np.mean(dust))


def full_path(item, index):
    """A 2x supersampled math frame at every decade; retain its .fds for contact."""
    with mp.workdps(max(90, int(55 + item['depth']))):
        c = item['nucleus']
        w0 = mp_scalar(item['origin'][3])
        to = item['size'] * 6
        depth = float(mp.log10(w0 / to))
        full = int(math.floor(depth))
        lengths = [1.0] * full + ([depth - full] if depth - full > 1e-4 else [])
        sigs, frames, widths = [], [], []
        for i, length in enumerate(lengths):
            w = to if i == len(lengths) - 1 else w0 / mp.power(10, i + 1)
            name = 'arc-c%d-d%03d' % (index, i + 1)
            cols, _ = render(mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps),
                             mp.nstr(w, 22), size=(256, 144), it=100000, name=name,
                             ss=2, columns='nu,de,normal')
            sigs.append(signature(cols))
            frames.append(TMP / (name + '.fds'))
            widths.append(mp.nstr(w, 22))
        return sigs, lengths, frames, widths


def make_candidate(item, sigs, lengths, frames, widths, result, out, minutes, settings):
    with mp.workdps(max(90, int(55 + item['depth']))):
        c, sz, origin = item['nucleus'], item['size'], item['origin']
        re, im = mp.nstr(mp.re(c), mp.mp.dps), mp.nstr(mp.im(c), mp.mp.dps)
        ending = mp.nstr(sz * 6, 24)
    out.mkdir(parents=True, exist_ok=True)
    speed = write_directed_speed(origin[3], ending, result['pace'], result['ramp'], out)
    rows = []
    for i, (sig, length) in enumerate(zip(sigs, lengths)):
        section = next(x['id'] for x in result['sections']
                       if x['first_decade'] <= i + 1 <= x['last_decade'])
        rows.append(dict(decade=i + 1, depth_start=i, depth_end=round(i + length, 5),
                         width=widths[i], signature=sig,
                         hero=round(hero_score(sig), 4),
                         busyness=round(busy_score(sig), 4),
                         section=section,
                         target=round(result['target'][i], 4),
                         achieved=round(result['achieved'][i], 4),
                         seconds=round(result['elapsed'][i], 3),
                         start_seconds=round(result['times'][i], 3),
                         end_seconds=round(result['times'][i + 1], 3)))
    plan = dict(re=re, im=im, to=ending, from_width=origin[3],
                region=origin[0], generation=item['generation'], spiral=item.get('spiral'),
                period=item['period'], size=mp.nstr(sz, 24),
                depth=item['depth'], minutes=minutes,
                score=result['score'], passes_gates=result['passes_gates'],
                mid_hero=result['mid_hero'], haze=result['haze'],
                filler_fraction=result['filler_fraction'], arc_rmse=result['arc_rmse'],
                sections=result['sections'], decades=rows,
                arc=dict(target=result['target'], achieved=result['achieved']),
                speed_file='speed.txt', inputs=settings,
                validity='score is heuristic; require visual inspection before full render')
    (out / 'plan.json').write_text(json.dumps(plan, indent=2))
    draw_arc(result['target'], result['achieved'], out)
    make_contact(rows, frames, out)
    return plan


def starting_regions(re, im, width, all_regions=True):
    """A root view for each classic family; each film remains a straight fixed-centre zoom."""
    roots = [('requested', str(re), str(im), str(width))]
    if all_regions:
        roots += [
            ('seahorse', '-0.74519683', '0.101869885', '0.020'),
            ('seahorse-spiral', '-0.743643887037151', '0.13182590420533', '0.008'),
            ('elephant', '0.27205033514905763', '0.006118038612346085', '0.012'),
            ('triple-spiral', '-0.0875937321188787', '0.6550902802386774', '0.015'),
            ('antenna', '-1.25066', '0.02012', '0.030'),
        ]
    return list(dict.fromkeys(roots))


def mp_scalar(value):
    """Accept NumPy scalar inputs without converting deep decimal strings via f64."""
    if isinstance(value, np.integer):
        value = int(value)
    elif isinstance(value, np.floating):
        value = str(value)
    return mp.mpf(value)


def mp_complex(value):
    """Convert all NumPy scalar kinds before entering mpmath."""
    if isinstance(value, np.complexfloating):
        return mp.mpc(mp_scalar(value.real), mp_scalar(value.imag))
    if isinstance(value, np.generic):
        return mp.mpc(mp_scalar(value))
    return mp.mpc(value)


def harvest_nuclei(re, im, width, rng, n):
    """One bounded tiny fd view; boundary seeds are Newton-solved to exact nuclei."""
    with mp.workdps(max(90, int(55 - mp.log10(mp_scalar(width))))):
        w = mp_scalar(width)
        cre, cim = mp_scalar(re), mp_scalar(im)
        cols, _ = render(mp.nstr(cre, mp.mp.dps), mp.nstr(cim, mp.mp.dps),
                         mp.nstr(w, 22), size=(160, 90), it=100000, name='harvest')
        ny, nx = cols['de'].shape
        pixel = w / nx
        found = []
        for x, y in candidates(cols, n, rng):
            # np.nonzero/argsort yield np.int64; convert at the grid boundary.
            x, y = int(x), int(y)
            c = mp.mpc(cre + pixel * (mp.mpf(x) + mp.mpf('0.5') - nx / 2),
                       cim - pixel * (mp.mpf(y) + mp.mpf('0.5') - ny / 2))
            result = nucleus(c, w, max_period=6000)
            if result:
                result = (mp_complex(result[0]), int(result[1]), mp_scalar(result[2]))
            if result and result[2] < w / 100 and abs(result[0] - mp.mpc(cre, cim)) < w * mp.mpf('0.6'):
                if not any(abs(result[0] - other[0]) < max(result[2], other[2]) * 3
                           for other in found):
                    found.append(result)
        return found


def search_targets(re, im, width, n, seed, minutes, min_decades, beam=4, rounds=3, regions=True):
    """Beam-search nested decorated minibrots; fail closed if 45+ decades cannot be found."""
    ideal = minutes * 60 * math.log10(1.25)
    low = max(min_decades, int(math.ceil(ideal * 0.76)))
    high = max(low + 8, int(math.ceil(ideal * 1.28)))
    rng = np.random.default_rng(seed)
    root_views = starting_regions(re, im, width, regions)
    leaves, seen = [], set()
    frontier = []
    def save(r, origin, generation):
        c, p, sz = r
        with mp.workdps(max(90, int(55 - mp.log10(mp_scalar(sz))))):
            c, p, sz = mp_complex(c), int(p), mp_scalar(sz)
            depth = float(mp.log10(mp_scalar(origin[3]) / (sz * 6)))
            key = (origin[0], mp.nstr(mp.re(c), mp.mp.dps),
                   mp.nstr(mp.im(c), mp.mp.dps))
        if not (0 < depth < high + 18) or key in seen:
            return None
        seen.add(key)
        item = dict(nucleus=c, period=p, size=sz, depth=depth,
                    origin=origin, generation=generation)
        if low <= depth <= high:
            leaves.append(item)
        return item

    if regions:
        # VIDE-01: period-764 nucleus, 100-digit verified coordinate.
        # Refine at 150 decimal digits so the address never crosses f64.
        with mp.workdps(150):
            vr = '-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
            vi = '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
            c0 = mp.mpc(mp.mpf(vr), mp.mpf(vi))
            calibrated = nucleus(c0, mp.mpf('1e-46'), max_period=2000)
            if calibrated is not None:
                save(calibrated, ('verified-v0', vr, vi, '4'), 0)
    for root in root_views:
        found = harvest_nuclei(root[1], root[2], root[3], rng, n)
        roots = [x for x in (save(r, root, 0) for r in found) if x is not None]
        roots.sort(key=lambda x: (abs(x['depth'] - ideal * 0.44), -x['period']))
        frontier += roots[:beam]
        print('  seed %-18s %2d nuclei (%d expandable)' %
              (root[0], len(found), min(len(roots), beam)), flush=True)
    for generation in range(1, rounds + 1):
        following = []
        for parent in frontier:
            with mp.workdps(max(90, int(55 - mp.log10(parent['size'])))):
                c, size = parent['nucleus'], parent['size']
                for factor in (300, 70):
                    window = size * factor
                    descendants = harvest_nuclei(mp.re(c), mp.im(c), window, rng, max(12, n // 2))
                    for child in descendants:
                        if child[2] >= size / 80:
                            continue
                        item = save(child, parent['origin'], generation)
                        if item is not None and item['depth'] < low and item['depth'] < high:
                            following.append(item)
        if not following:
            print('  generation %d: no new deeper nuclei' % generation, flush=True)
            break
        # Keep multiple origins and periods alive; never narrow to one lucky branch.
        groups = {}
        for item in following:
            groups.setdefault(item['origin'][0], []).append(item)
        frontier = []
        for name, group in groups.items():
            group.sort(key=lambda x: (abs(x['depth'] - ideal * 0.76), -x['period']))
            frontier += group[:beam]
        print('  generation %d: %d new targets in depth band %d..%d, %d parents' %
              (generation, len(leaves), low, high, len(frontier)), flush=True)
    # Stable determinism, and no shallow fallback that stretches 27 decades into 600 s.
    leaves.sort(key=lambda x: (abs(x['depth'] - ideal), x['origin'][0], x['period']))
    return leaves, (low, high)


def plan_directed(re, im, width, out, minutes, spiral=None, n=24, seed=1,
                  min_decades=20, count=6, beam=4, rounds=3, regions=True,
                  settings=None):
    """Discover, cheaply triage, then fully score six diverse fixed-centre films."""
    start = time.time()
    roots, band = search_targets(re, im, width, n, seed, minutes, min_decades,
                                 beam=beam, rounds=rounds, regions=regions)
    print('%d deep candidates in %d..%d decades (%.1fs)' %
          (len(roots), *band, time.time() - start), flush=True)
    if not roots:
        raise SystemExit('no sufficiently deep target; increase --beam/--rounds/--cands; no shallow-duration fallback')
    # Limit expensive probes, while sampling each region and preserving diverse periods.
    by_region = {}
    for item in roots:
        by_region.setdefault(item['origin'][0], []).append(item)
    pending = []
    for group in by_region.values():
        pending.extend(group[:max(count * 2, 9)])
    ranked = []
    for i, item in enumerate(pending):
        if not visible(item['nucleus'], item['size']):
            continue
        item['probe'] = coarse_score(item)
        ranked.append(item)
        print('  probe %2d/%d %-18s depth %.1f score %+.2f' %
              (i + 1, len(pending), item['origin'][0], item['depth'], item['probe']), flush=True)
    if not ranked:
        raise SystemExit('no visible targets passed the minibrot probe')
    ranked.sort(key=lambda x: -x['probe'])
    shortlist, counts = [], {}
    for item in ranked:
        name = item['origin'][0]
        if counts.get(name, 0) >= max(2, (count + 1) // 2):
            continue
        counts[name] = counts.get(name, 0) + 1
        shortlist.append(item)
        if len(shortlist) >= count + 3:
            break
    finalists = []
    for i, item in enumerate(shortlist):
        sigs, lengths, frames, widths = full_path(item, i)
        try:
            result = assess(sigs, lengths, minutes)
        except ValueError as error:
            print('  discard %s depth %.1f: %s' % (item['origin'][0], item['depth'], error),
                  flush=True)
            continue
        result['score'] -= 0.4 * abs(item['depth'] - minutes * 60 * math.log10(1.25)) / 10
        item['spiral'] = None
        if spiral:
            d1, d2 = spiral
            if not (1 <= d1 < d2 < item['depth'] - 1):
                continue
            w2 = mp_scalar(item['origin'][3]) / mp.power(10, d2)
            centre = spiral_near(item['nucleus'], mp_scalar(item['origin'][3]) / mp.power(10, d1))
            if (centre is None or centre['turns'] < 1.5
                    or abs(item['nucleus'] - centre['c']) > w2 * mp.mpf('0.06')):
                continue
            item['spiral'] = dict(decades=[d1, d2], turns=centre['turns'],
                                  max_offset_fraction=float(abs(item['nucleus'] - centre['c']) / w2))
        finalists.append((result['score'], item, sigs, lengths, frames, widths, result))
        print('  final %-18s depth %.1f score %+.2f mid hero %.2f haze %.2f filler %.0f%%' %
              (item['origin'][0], item['depth'], result['score'],
               result['mid_hero'], result['haze'], 100 * result['filler_fraction']), flush=True)
    finalists.sort(key=lambda x: -x[0])
    path = Path(out)
    path.mkdir(parents=True, exist_ok=True)
    listing = []
    for i, (_, item, sigs, lengths, frames, widths, result) in enumerate(finalists[:count], 1):
        candidate = path / ('cand-%d' % i)
        plan = make_candidate(item, sigs, lengths, frames, widths, result,
                              candidate, minutes, settings)
        listing.append(dict(pick=i, path=str(candidate), score=result['score'],
                            region=plan['region'], period=plan['period'],
                            depth=round(plan['depth'], 2), sections=len(plan['sections']),
                            mid_hero=round(plan['mid_hero'], 3),
                            filler_fraction=round(plan['filler_fraction'], 3),
                            haze=round(plan['haze'], 3),
                            passes_gates=plan['passes_gates']))
    if not listing:
        raise SystemExit('no candidates could meet the 3-6 decade section and spiral constraints')
    (path / 'ranking.json').write_text(json.dumps(listing, indent=2))
    (path / 'plan.json').write_text(json.dumps(dict(ranked=listing, inputs=settings), indent=2))
    for x in listing:
        print('  PICK %d %-18s D%4.1f score %+.2f %s -> %s' %
              (x['pick'], x['region'], x['depth'], x['score'],
               'passes' if x['passes_gates'] else 'INSPECT/BELOW GATES',
               x['path']), flush=True)
    print('planned %d films in %.1fs; pick by contact/arc before rendering' %
          (len(listing), time.time() - start), flush=True)
    return listing


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


def make_contact(rows, frames, out):
    """Reuse 2x supersampled scoring frames; match film's unresolved-as-interior."""
    images = []
    for row, f in zip(rows, frames):
        subprocess.run([FD, 'shade', str(f), '--preset', 'master-ice',
                        '--unresolved', 'interior', '--aa', 'on', '-o', str(TMP)],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        images.append((TMP / (f.stem + '.studio.png'), row))
    cmd = ['montage', '-font', 'DejaVu-Sans', '-pointsize', '13', '-background', '#171b24',
           '-fill', 'white', '-tile', '4x', '-geometry', '384x216+5+5']
    for img, row in images:
        cmd += ['-label', 'D%02d H%.2f S%d unres %.0f%%' %
                (row['decade'], row['hero'], row['section'],
                 100 * row['signature']['unresolved']), str(img)]
    subprocess.run(cmd + [str(out / 'contact.jpg')], check=True, stdout=subprocess.DEVNULL)


def run_film(final, out, fps, size, preset, preview_seconds=None, preview_at=0.65):
    """Render a chosen plan only, preserving the same 600 s speed curve in previews."""
    root = Path(out)
    movie = root / ('preview.mp4' if preview_seconds else 'film.mp4')
    cmd = [FD, 'film', final['re'], final['im'],
           '--from', final['from_width'], '--to', final['to'],
           '--speed', str(root / 'speed.txt'), '--fps', str(fps), '--size', size,
           '--keyframes', '2', '--gpu', 'on', '--preset', preset, '--mp4', str(movie)]
    if preview_seconds:
        total = round(final['minutes'] * 60 * fps)
        count = min(total, round(preview_seconds * fps))
        first = min(round(preview_at * total), total - count)
        cmd += ['--frames', '%d..%d' % (first, first + count)]
    print('film command:', ' '.join(cmd), flush=True)
    start = time.time()
    r = subprocess.run(cmd, check=True, stdout=subprocess.PIPE, text=True)
    wall = time.time() - start
    (root / ('preview-report.json' if preview_seconds else 'film-report.json')).write_text(r.stdout)
    try:
        report = json.loads(r.stdout)
        frames = report.get('frames', round((preview_seconds or final['minutes'] * 60) * fps))
        wall = report.get('wall_seconds', wall)
    except (ValueError, TypeError):
        frames = round((preview_seconds or final['minutes'] * 60) * fps)
    print('film: %s, %d frames, %.1fs wall, %.3f fps' %
          (movie, frames, wall, frames / wall), flush=True)


if __name__ == '__main__':
    main()
