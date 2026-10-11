#!/usr/bin/env python3
"""PROB-21: shape definitions from Hubbard trees at Misiurewicz points (research tool).

  hubbard.py --point RE IM --out DIR      one point: angles, arms, multiplier, tree -> DIR/point.json
  hubbard.py --catalog --out DIR          corpus + per-shape views + render.sh + contact-sheet.md

Per Misiurewicz point c0 (preperiod q from z_1 = c, period r):
- its external angles (exact rationals) and so its arm count k (rays landing at c0);
- the cycle multiplier lambda = (f^r)'(z_q) and the combinatorial rotation p/k of the rays at
  the cycle point, which together give the residual twist delta = arg(lambda e^{-2 pi i p/k}):
  the actual turn of the arms per self-similar step (rotation by p/k only swaps arms);
- the Hubbard tree: exact topology (bks.py), regulated arcs traced in the plane (geom.py);
- Tan Lei's scale S, so a dynamical-plane picture near c0 maps to the parameter plane.
Views use one rule for every point, so a shape test never reduces to "how far zoomed in".
"""
import argparse
import json
import math
import os
import shlex
import sys
from fractions import Fraction
from multiprocessing import Pool
from pathlib import Path

import mpmath as mp
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import geom  # noqa: E402

DPS = 60
V0 = ('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502',
      '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')
VIEW_SPAN = 80  # a view shows structure from its half-width down to about 2 pixels of 320
SHAPES = ('hook', 'spoke/backbone', 'dendrite', 'whirlpool', 'galaxy', 'tendril', 'filament', 'spindle')
NOT_DEFINABLE = {
    'filament': 'an edge between two forks needs a view framing that edge, and the Tan Lei image of a '
                'whole tree edge is too large for the similarity to hold (such views land on bulbs). It needs '
                'the tree pulled back to view scale by the cycle\'s inverse branches: a follow-up.',
    'spindle': 'an antenna segment between consecutive minibrots: needs minibrot (hyperbolic '
               'component) detection, which MAP-01 adds; between forks alone it is a filament.',
}


def dec(x, digits=DPS - 5):
    return mp.nstr(x, digits, min_fixed=-mp.inf, max_fixed=mp.inf)


# ---------------------------------------------------------------- one point

def rotation(thetas, q, r):
    """Combinatorial rotation number p/k of the rays at the cycle point z_q."""
    k = len(thetas)
    at_cycle = sorted({(t * 2 ** (q - 1)) % 1 for t in thetas})
    if len(at_cycle) != k:
        raise ValueError('rays at c0 do not map one-to-one to the cycle point: decline')
    image = (at_cycle[0] * 2 ** r) % 1
    if image not in at_cycle:
        raise ValueError('rays at the cycle point are not permuted: decline')
    return at_cycle.index(image), k


def measure(c0, thetas, q, r, tree=True):
    """All per-point quantities; c0 an mp complex at DPS."""
    with mp.workdps(DPS):
        _, lam, S = geom.local_data(c0, q, r, DPS)
    p, k = rotation(thetas, q, r)
    if k > 2 and k * r != geom.angle_type(min(thetas))[1]:
        raise ValueError('arm count disagrees with the ray period (Milnor): decline')
    lam_c = complex(lam)
    delta = math.remainder(math.atan2(lam_c.imag, lam_c.real) - 2 * math.pi * p / k, 2 * math.pi)
    levels = math.log(VIEW_SPAN) / math.log(abs(lam_c))
    with mp.workdps(DPS):
        z, _ = geom.orbit(c0, q + r)
    near = min((complex(z[j] - z[1]) for j in range(q + r) if j != 1), key=abs)
    out = dict(re=dec(mp.re(c0)), im=dec(mp.im(c0)), q=q, r=r, near_dyn=[near.real, near.imag],
               angles=[str(t) for t in sorted(thetas)], arms=k, rotation=f'{p}/{k}',
               multiplier=[lam_c.real, lam_c.imag], abs_multiplier=abs(lam_c),
               twist_deg=math.degrees(delta), levels_in_view=levels,
               turns_in_view=levels * delta / (2 * math.pi),
               scale=[complex(S).real, complex(S).imag])
    if tree:
        try:
            T = geom.HubbardTree(complex(c0), min(thetas), q, r)
            st = T.structure()
            if st['spread'] > 5e-3:
                raise ValueError('tree placement inconsistent with its combinatorics')
        except ValueError as e:
            # Weakly repelling cycles (|lambda| near 1) spiral too slowly for the arcs to settle;
            # such points stay in the corpus for the centre views, which need no tree.
            out.update(tree_error=str(e), edge_turn_deg=None)
            return out
        v1 = st['first_vertex']
        out.update(tree_nodes=st['nodes'], tree_depth=T.depth, tree_residual=T.residual,
                   tree_spread=st['spread'],
                   branch_arms=sorted((b['arms'] for b in st['branch']), reverse=True),
                   edge_dyn=[complex(v1 - T.c).real, complex(v1 - T.c).imag],
                   edge_ends_at_fork=bool(st['first_is_branch']),
                   edge_turn_deg=math.degrees(geom.turning(st['edge'])))
    return out


def point(re, im, max_mult=6):
    """--point: refine the Misiurewicz point, find its angles by outward rays, measure it."""
    with mp.workdps(DPS + 60):
        c = mp.mpc(re, im)
        q, n = discover(c)
        c, q, r = geom.misiurewicz(c, q, n, DPS + 60, mp.mpf('1e-3'))
        _, lam, S = geom.local_data(c, q, r, DPS + 60)
        found = geom.angles_at(c, q, r, S, lam, starts=24, cycles=4, max_mult=max_mult)
        if not found:
            raise ValueError('no external angle recovered: decline')
        thetas = [t for t in found if geom.angle_type(t)[1] == geom.angle_type(max(found, key=found.get))[1]]
        out = measure(c, thetas, q, r)
        out['angle_votes'] = {str(t): v for t, v in found.items()}
        return out


def discover(c, max_q=64, max_n=12):
    """Smallest (q, n) with z_{q+n} ~ z_q at c (a seed for Newton)."""
    z, _ = geom.orbit(c, max_q + max_n)
    return min(((abs(z[q + n] - z[q]) / (1 + abs(z[q])), q, n) for q in range(2, max_q + 1)
                for n in range(1, max_n + 1)), key=lambda x: (x[0] > mp.mpf('1e-12'), x[1] + x[2], x[0]))[1:]


# ---------------------------------------------------------------- corpus

def corpus_angles(max_pre, max_period):
    """Exact angles with ray preperiod 1..max_pre and period 1..max_period, upper half plane."""
    out = []
    for qa in range(1, max_pre + 1):
        for n in range(1, max_period + 1):
            den = 2 ** qa * (2 ** n - 1)
            for a in range(1, den):
                t = Fraction(a, den)
                if geom.angle_type(t) == (qa, n) and t <= Fraction(1, 2):
                    out.append(t)
    return sorted(set(out))


WAKES = ((Fraction(1, 3), Fraction(2, 3)), (Fraction(1, 7), Fraction(2, 7)), (Fraction(1, 15), Fraction(2, 15)),
         (Fraction(1, 31), Fraction(2, 31)), (Fraction(9, 31), Fraction(10, 31)), (Fraction(1, 63), Fraction(2, 63)))


def root_angles(m_max=28):
    """Rays crowding a bulb's root from both sides of its wake (a, b), a la v0 (1/3 - 1/(3*2^23)).

    Inside the wake they land on Misiurewicz points mapping onto alpha (q arms for a p/q bulb);
    outside, on the bulb's own cycle. Near the root that cycle is barely repelling, so many
    nested self-similar levels fit in one view (spirals, galaxies, whirlpools).
    """
    out = set()
    for a, b in WAKES:
        for m in range(4, m_max + 1):
            e = Fraction(1, 2 ** m)
            for t in (a - a * e, b + (1 - b) * e, a + (b - a) * e, b - (b - a) * e):
                if 0 < t <= Fraction(1, 2) and geom.angle_type(t)[0] > 0:
                    out.add(t)
    return sorted(out)


def land(thetas, depth=96):
    """Trace rays in, Newton to the Misiurewicz point, group rays by landing point."""
    ends = geom.rays_in(thetas, depth=depth)
    points = {}
    for t, e in zip(thetas, ends):
        qa, n = geom.angle_type(t)
        try:
            with mp.workdps(DPS):
                c, q, r = geom.misiurewicz(mp.mpc(e.real, e.imag), qa + 1, n, DPS, mp.mpf('1e-6'))
        except (ValueError, ZeroDivisionError):
            continue
        if q != qa + 1 or mp.im(c) < -mp.mpf('1e-30'):
            continue
        key = (q, r, mp.nstr(mp.re(c), 25), mp.nstr(abs(mp.im(c)), 25))
        points.setdefault(key, dict(c=c, q=q, r=r, thetas=set()))['thetas'].add(t)
        if abs(mp.im(c)) < mp.mpf('1e-30'):  # real point: its conjugate ray lands here too
            points[key]['thetas'].add((1 - t) % 1)
    for p in points.values():
        for t in list(p['thetas']):
            p['thetas'] |= partners(t, p['q'], p['r'])
    return list(points.values())


def partners(t, q, r):
    """All rays landing with ray t at its Misiurewicz point, when the rays there rotate (n > r).

    The rays at the cycle point are the orbit of 2^(q-1) t under 2^r (Milnor: more than one
    ray at a rotating cycle point are permuted transitively). Each is pulled back to c along
    t's own kneading halves, since all rays at a postcritical point lie in the same half.
    """
    n = geom.angle_type(t)[1]
    if n <= r:
        return {t}
    lo, hi = t / 2, (t + 1) / 2
    sides = []
    a = t
    for _ in range(q - 1):  # half of z_1 .. z_{q-1}
        sides.append(lo < a < hi)
        a = 2 * a % 1
    phi = (t * 2 ** (q - 1)) % 1
    out = set()
    for i in range(n // r):
        x = (phi * 2 ** (r * i)) % 1
        for side in reversed(sides):
            x = next(y for y in (x / 2, (x + 1) / 2) if (lo < y < hi) == side)
        out.add(x)
    return out


def _measure_job(job):
    try:
        with mp.workdps(DPS):
            c = mp.mpc(job['re'], job['im'])
        out = measure(c, [Fraction(t) for t in job['thetas']], job['q'], job['r'], job.get('tree', True))
        out['name'] = job['name']
        return out
    except (ValueError, ZeroDivisionError, OverflowError, KeyError) as e:
        return dict(name=job['name'], declined=str(e))


def corpus(max_pre=5, max_period=6, trees=400, workers=None):
    thetas = corpus_angles(max_pre, max_period)
    near_roots = [t for t in root_angles() if t not in set(thetas)]
    landed = land(thetas) + land(near_roots, depth=320)  # rays near a root approach slowly
    thetas += near_roots
    jobs = [dict(name='M%d,%d-%s' % (p['q'], p['r'], str(min(p['thetas'])).replace('/', '_')),
                 re=dec(mp.re(p['c'])), im=dec(mp.im(p['c'])), q=p['q'], r=p['r'],
                 thetas=[str(t) for t in p['thetas']]) for p in landed]
    with mp.workdps(DPS + 60):
        c, q, r = geom.misiurewicz(mp.mpc(*V0), 24, 2, DPS + 60, mp.mpf('1e-20'))
    jobs.append(dict(name='v0-M24,2', re=dec(mp.re(c)), im=dec(mp.im(c)), q=q, r=r,
                     thetas=['8388607/25165824']))  # found by hubbard.py --point (ray_out)
    for j in jobs:
        j['tree'] = False
    with Pool(workers or os.cpu_count()) as pool:
        cases = pool.map(_measure_job, jobs, chunksize=4)
        # Trees (reported per point; no shape test needs them) on a fixed sample: every multi-armed or deeply nested
        # point, then an even spread of the rest, up to `trees` points.
        ok = [c for c in cases if 'declined' not in c]
        first = [c for c in ok if c['arms'] >= 3 or c['levels_in_view'] >= 15]
        rest = [c for c in ok if c not in first]
        sample = first + rest[::max(1, len(rest) // max(1, trees - len(first)))]
        sample = {c['name'] for c in sample[:trees]}
        for j in jobs:
            j['tree'] = j['name'] in sample
        treed = {c['name']: c for c in pool.map(_measure_job, [j for j in jobs if j['tree']], chunksize=1)}
    cases = [treed.get(c['name'], c) for c in cases]
    return [c for c in cases if 'declined' not in c], [c for c in cases if 'declined' in c], len(thetas)


# ---------------------------------------------------------------- views and shape tests

def centre_view(case):
    """Centred on c0, sized to the local self-similar structure there (one rule for every point).

    The structure lives at the cycle point z_q: radius = distance to the nearest other postcritical
    point times min(0.3, |lambda| - 1) (the cycle's linearisation shrinks as |lambda| -> 1). It is
    carried back to c by the derivative of f^(q-1) along z_1..z_{q-1} (long preperiods that linger
    near a weak cycle make it tiny) and to the parameter plane by Tan Lei's scale S.
    """
    with mp.workdps(DPS):
        z, _ = geom.orbit(mp.mpc(case['re'], case['im']), case['q'] + case['r'])
        q = case['q']
        rq = min(abs(z[j] - z[q]) for j in range(1, len(z)) if j != q and abs(z[j] - z[q]) > 0)
        back = mp.fprod(abs(2 * z[j]) for j in range(1, q))
        local = rq * min(0.3, case['abs_multiplier'] - 1) / back
    return dict(re=case['re'], im=case['im'], width=float(abs(complex(*case['scale'])) * local), kind='centre')


# Each test: (view, predicate, margin, plain-words threshold). All thresholds are new proposals.
def _turns(c):
    return abs(c['turns_in_view'])


TESTS = {
    'hook': (centre_view, lambda c: c['arms'] == 1 and 0.4 <= _turns(c) < 1.2 and c['levels_in_view'] < 8,
             lambda c: min(1.0 if c['arms'] == 1 else -1.0, _turns(c) - 0.4, 1.2 - _turns(c),
                           (8 - c['levels_in_view']) / 8),
             'k = 1 arm at c0 (it ends there) making one loose curl: 0.4 to 1.2 turns and fewer than 8 '
             'nested levels in view (tighter, many-level curls are galaxies)'),
    'tendril': (centre_view, lambda c: c['arms'] == 2 and _turns(c) >= 1.1,
                lambda c: min(1.0 if c['arms'] == 2 else -1.0, _turns(c) - 1.1),
                'k = 2 arms at c0 (a strand passing through) twisting >= 1.1 turns across the view: '
                'an edge that accumulates rotation without ending (the issue\'s "double hook")'),
    'spoke/backbone': (centre_view, lambda c: c['arms'] <= 2 and _turns(c) < 0.1,
                       lambda c: min(1.0 if c['arms'] <= 2 else -1.0, 0.1 - _turns(c)),
                       'k <= 2 arms at c0 and they twist < 0.1 turn across the view (a straight spine)'),
    'dendrite': (centre_view, lambda c: c['arms'] >= 3 and _turns(c) < 0.15,
                 lambda c: min(1.0 if c['arms'] >= 3 else -1.0, 0.15 - _turns(c)),
                 'k >= 3 arms at c0 and they twist < 0.15 turn across the view (non-rotating fork)'),
    'whirlpool': (centre_view, lambda c: c['arms'] >= 3 and _turns(c) >= 0.3,
                  lambda c: min(1.0 if c['arms'] >= 3 else -1.0, _turns(c) - 0.3),
                  'k >= 3 arms at c0 and they twist >= 0.3 turn across the view'),
    'galaxy': (centre_view, lambda c: c['levels_in_view'] >= 15 and _turns(c) >= 1.0,
               lambda c: min(c['levels_in_view'] - 15, _turns(c) - 1.0),
               '>= 15 nested self-similar levels in the view (|lambda| <= 1.34) twisting >= 1 turn'),
}


def pick(cases, shape, count=6):
    """Six firing and six non-firing views, each at a distinct point, spread over the corpus.

    Fires: strongest margins first. Does not fire: half are the nearest misses, half spread
    across the rest, so the "no" set is not just the easy cases.
    """
    view, test, margin, _ = TESTS[shape]
    scored = sorted(cases, key=margin, reverse=True)
    yes = [c for c in scored if test(c)]
    no = [c for c in scored if not test(c)]
    near, rest = no[:count // 2], no[count // 2:]
    step = max(1, len(rest) // (count - len(near))) if rest else 1
    no_pick = near + rest[::step][:count - len(near)]
    out = []
    for fired, group in ((True, yes[:count]), (False, no_pick)):
        for i, c in enumerate(group):
            v = view(c)
            v.update(shape=shape, fires=fired, case=c['name'], margin=float(margin(c)),
                     stem='%s-%s-%02d' % (shape.replace('/', '-'), 'yes' if fired else 'no', i + 1))
            out.append(v)
    return out


# ---------------------------------------------------------------- schematics

# What each shape should look like, drawn from the same model the tests use: k arms, each a
# logarithmic spiral that turns `turns` times across the view's levels, with side twigs for forks
# and small copies of the centre along the arm for nested levels. (arms, turns, twigs, nested)
SCHEMATIC = {
    'hook': [(1, 0.5, False, False), (1, 0.8, False, False), (1, 1.1, False, False)],
    'tendril': [(2, 0.5, False, False), (2, 1.0, False, False), (2, 1.6, False, False)],
    'spoke/backbone': [(1, 0.0, True, False), (2, 0.0, True, False), (2, 0.06, False, False)],
    'dendrite': [(3, 0.0, True, False), (4, 0.03, True, False), (5, 0.0, True, False)],
    'whirlpool': [(3, 0.6, False, False), (4, 0.8, False, False), (6, 0.6, False, False)],
    'galaxy': [(1, 2.5, False, True), (2, 2.0, False, True), (1, 3.5, False, True)],
}
# Plain words for the owner: what to look for, and what it is not.
DESCRIBE = {
    'hook': ('ONE arm that comes in and ends at the centre, curling around it once as it ends (a '
             'shepherd\'s crook). A single loose curl, not a tight many-turn spiral.',
             'not a tight spiral with nested spirals (galaxy), not two arms passing through (tendril), '
             'not a straight tip (spoke).'),
    'tendril': ('TWO arms through the centre, both curling, so the strand makes an S or a double '
                'curl as it passes through.',
                'not one arm that ends (hook), not a straight line (spoke), not 3+ arms (whirlpool).'),
    'spoke/backbone': ('ONE or TWO arms that run STRAIGHT into or through the centre: a straight spine or '
                       'antenna, usually with small side branches along it.',
                       'not curling (hook/tendril), not 3+ arms meeting (dendrite).'),
    'dendrite': ('THREE OR MORE arms meeting at the centre, all nearly STRAIGHT, like a star, a cross or a '
                 'branching twig. 3, 4, 5 ... arms all count; what matters is "several straight arms meet here".',
                 'not swirling (whirlpool), not just one or two arms (spoke).'),
    'whirlpool': ('THREE OR MORE arms meeting at the centre, all SWIRLING the same way around it, like '
                  'water going down a drain or a pinwheel.',
                  'not straight arms (dendrite), not just one or two arms (hook/tendril).'),
    'galaxy': ('A TIGHT spiral wound many times around the centre, with smaller spirals repeating along '
               'its arms, like a spiral galaxy. Any number of arms; the many turns and nested repeats matter.',
               'not a single loose curl (hook), not straight arms.'),
}


def _arm(phi, turns, scale=1.0, cx=0.0, cy=0.0, s0=-4.5):
    pts = []
    for i in range(121):
        s = s0 * (1 - i / 120)  # log radius from deep inside out to the frame
        a = phi + 2 * math.pi * turns * (s - s0) / -s0
        pts.append((cx + scale * math.exp(s) * math.cos(a), cy + scale * math.exp(s) * math.sin(a)))
    return pts


def schematic(shape, variant=0, size=180):
    k, turns, twigs, nested = SCHEMATIC[shape][variant]
    paths = []
    for j in range(k):
        arm = _arm(2 * math.pi * j / k, turns)
        paths.append(arm)
        if twigs:  # forks: short side branches, shrinking toward the centre
            for i in range(30, 121, 18):
                (x0, y0), (x1, y1) = arm[i - 1], arm[i]
                d = math.atan2(y1 - y0, x1 - x0)
                r = math.hypot(x1, y1) * 0.35
                for side in (1, -1):
                    paths.append([(x1, y1), (x1 + r * math.cos(d + side * 1.0), y1 + r * math.sin(d + side * 1.0))])
        if nested:  # smaller copies of the centre spiral along the arm
            for i in (70, 92, 108):
                x, y = arm[i]
                sc = math.hypot(x, y) * 0.22
                paths.append(_arm(math.atan2(y, x) + 2.5, 1.6, sc, x, y, s0=-3))
    h = size / 2
    d = ' '.join('M' + ' L'.join('%.1f,%.1f' % (h + 0.95 * h * x, h - 0.95 * h * y) for x, y in pts) for pts in paths)
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="0 0 %d %d">'
            '<rect width="100%%" height="100%%" fill="white"/><path d="%s" fill="none" stroke="#1b2a4a" '
            'stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>'
            '<circle cx="%.1f" cy="%.1f" r="3" fill="#d33"/></svg>') % (size, size, size, size, d, h, h)


# ---------------------------------------------------------------- output

def write(cases, declined, n_angles, out):
    out = Path(out)
    (out / 'img').mkdir(parents=True, exist_ok=True)
    views = [v for s in TESTS for v in pick(cases, s)]
    shown = {v['case'] for v in views}
    summary = dict(angles=n_angles, measured=len(cases), declined=len(declined),
                   declined_reasons=sorted({d['declined'] for d in declined}),
                   arms={str(k): sum(c['arms'] == k for c in cases) for k in sorted({c['arms'] for c in cases})},
                   with_tree=sum(c.get('edge_turn_deg') is not None for c in cases),
                   fires={s: sum(TESTS[s][1](c) for c in cases) for s in TESTS})
    (out / 'cases.json').write_text(json.dumps(dict(corpus=summary, shown=[c for c in cases if c['name'] in shown]),
                                               indent=1) + '\n')
    (out / 'views.json').write_text(json.dumps(views, indent=1) + '\n')
    lines = ['#!/usr/bin/env bash', '# Renders the PROB-21 contact sheet thumbnails (generated by hubbard.py).',
             'set -euo pipefail', 'FD="${FD:-target/release/fd}"', 'cd "$(dirname "$0")"',
             'tmp=$(mktemp -d); trap \'rm -rf "$tmp"\' EXIT']
    for v in views:
        lines.append('"$FD" render --re %s --im %s --width %s --size 320x180 --ss 3 --iter 100000 '
                     '--columns nu,de,normal -o "$tmp/%s.fds" >/dev/null 2>&1' % (v['re'], v['im'], repr(v['width']), v['stem']))
        lines.append('"$FD" shade "$tmp/%s.fds" --preset ice -o "$tmp" >/dev/null 2>&1' % v['stem'])
        lines.append('convert "$tmp/%s.studio.png" -quality 82 img/%s.jpg' % (v['stem'], v['stem']))
    (out / 'render.sh').write_text('\n'.join(lines) + '\n')
    (out / 'render.sh').chmod(0o755)
    by = {c['name']: c for c in cases}
    md = ['# PROB-21 contact sheet: shapes as Hubbard-tree predicates', '',
          'Generated by `python3 tools/research/shapes/hubbard.py --catalog --out docs/research/shapes`;',
          'thumbnails by `bash docs/research/shapes/render.sh` (fd, 320x180, 3x3 supersampled).', '',
          f'Corpus: {n_angles} exact external angles traced to {len(cases) + len(declined)} Misiurewicz '
          f'points; {len(cases)} measured, {len(declined)} declined (see cases.json). '
          'Every view of a kind uses the same zoom rule, so "fires" never just means "zoomed in".', '',
          '**How to read a row.** k = arms at the centre (rays landing there); twist = turn of the arms '
          'per self-similar step after removing the rays\' combinatorial rotation; levels = self-similar '
          'steps between the view\'s half-width and 2 pixels; turns = twist times levels. All views are '
          'centred on the point. In each "does not fire" column rows 1-3 are the nearest misses (they test '
          'where the threshold sits) and rows 4-6 are spread over the rest of the corpus.', '',
          '**Owner:** mark each shape below *keep* or *drop*, and any thumbnail that looks wrong.', '']
    for shape in SHAPES:
        md += [f'## {shape}', '']
        if shape in NOT_DEFINABLE:
            md += ['**Not definable yet:** ' + NOT_DEFINABLE[shape], '']
            continue
        look, notit = DESCRIBE[shape]
        svgs = []
        for i in range(len(SCHEMATIC[shape])):
            svgs.append('schematic-%s-%d.svg' % (shape.replace('/', '-'), i + 1))
            (out / 'img' / svgs[-1]).write_text(schematic(shape, i))
        md += ['**Look for:** ' + look, '', '**Not it:** ' + notit, '',
               '**Should look like** (schematics from the model, a few variants; red dot = the centre point):', '',
               ' '.join(f'![](img/{f})' for f in svgs), '',
               '**Test (new):** ' + TESTS[shape][3] + '.', '',
               '| | fires | does not fire |', '|---|---|---|']
        vs = [v for v in views if v['shape'] == shape]
        yes = [v for v in vs if v['fires']]
        no = [v for v in vs if not v['fires']]
        for i in range(max(len(yes), len(no))):
            cells = []
            for v in (yes[i] if i < len(yes) else None, no[i] if i < len(no) else None):
                if v is None:
                    cells.append('(none in corpus)')
                    continue
                c = by[v['case']]
                cells.append('![](img/%s.jpg)<br>`%s` k=%d twist=%.0f° turns=%.2f levels=%.1f' % (
                    v['stem'], c['name'], c['arms'], c['twist_deg'], c['turns_in_view'], c['levels_in_view']))
            md.append('| %d | %s | %s |' % (i + 1, cells[0], cells[1]))
        md.append('')
    md += ['## Corpus', '', 'Points per arm count: ' + ', '.join(f'k={k}: {v}' for k, v in summary['arms'].items())
           + '. Points where each test fires: ' + ', '.join(f'{s}: {v}' for s, v in summary['fires'].items()) + '.', '',
           'Exact decimal centres and widths for every thumbnail are in `views.json`; render any of them with '
           '`fd render --re RE --im IM --width W`.', '',
           '## Established vs new', '',
           '**Established** (used as is): Hubbard trees as regulated arcs joining the critical orbit '
           '(Douady-Hubbard, Orsay notes); the kneading/triod construction of their topology '
           '(Bruin-Kaffl-Schleicher 2009, Prop. 3.5); landing of rational parameter rays at Misiurewicz '
           'points and transitive rotation of more than one ray at a cycle point (Douady-Hubbard; Milnor, '
           '"Periodic orbits, external rays and the Mandelbrot set"); asymptotic similarity of M and J_c0 '
           'at a Misiurewicz point (Tan Lei 1990); ray tracing in and out (Heiland-Allen, mandelbrot-numerics). '
           'The shape names come from Munafo\'s Mu-Ency.', '',
           '**New** (proposals for the owner to judge): every shape test above and its threshold; the '
           '*residual twist* (multiplier argument minus the rays\' combinatorial rotation) as the visible arm '
           'turn per self-similar level; counting turns over a fixed span of levels per view; the zoom rules '
           '(centre: Tan Lei image of the distance to the nearest postcritical point;'
           'placing tree vertices as triod centres of numerically lifted arcs.', '',
           '**Not certified:** ray landing is numerical (Newton from the traced ray end, with the arm count '
           'checked against Milnor\'s rule); tree arcs are converged numerically; Tan Lei\'s similarity is '
           'asymptotic (it held in the views checked; edge-scale views did not hold, hence filament).', '']
    (out / 'contact-sheet.md').write_text('\n'.join(md))
    return views


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--point', nargs=2, metavar=('RE', 'IM'))
    ap.add_argument('--catalog', action='store_true')
    ap.add_argument('--out', required=True)
    ap.add_argument('--max-pre', type=int, default=5)
    ap.add_argument('--max-period', type=int, default=6)
    ap.add_argument('--trees', type=int, default=400)
    a = ap.parse_args(argv)
    if bool(a.point) == a.catalog:
        ap.error('give exactly one of --point RE IM or --catalog')
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    try:
        if a.point:
            res = point(*a.point)
            (out / 'point.json').write_text(json.dumps(res, indent=1) + '\n')
            print(json.dumps({k: res[k] for k in ('q', 'r', 'angles', 'arms', 'rotation', 'twist_deg',
                                                    'abs_multiplier', 'branch_arms')}))
        else:
            cases, declined, n = corpus(a.max_pre, a.max_period, a.trees)
            views = write(cases, declined, n, out)
            print(f'{n} angles, {len(cases)} points measured, {len(declined)} declined, {len(views)} views')
    except (ValueError, ZeroDivisionError) as e:
        ap.error(str(e))


if __name__ == '__main__':
    main()
