#!/usr/bin/env python3
"""Research only: normalized parameter jets; no outward-rounded certificate.
Freeze direct mpmath truth separately, then compare without an oracle in the timing.
"""
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path
import statistics
import time
import mpmath as mp

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CENTRE = (ROOT / 'bench/path-atlas-v0.txt').read_text().splitlines()
V0 = next(line.split()[:2] for line in CENTRE if line and not line.startswith('#'))
SCENES = [
    ('cusp-6', '0.25', '0', '1e-6'),
    ('cusp-8', '0.25', '0', '1e-8'),
    ('valley-14', '-0.743643887037158704752191506114774',
     '0.131825904205311970493132056385139', '1e-14'),
    ('v0-14', *V0, '1e-14'),
    ('v0-29', *V0, '1e-29'),
    ('i-29', '0', '1', '1e-29'),
    ('tip-29', '-2', '0', '1e-29'),
]
OFFSETS = [('0.4', '0.2'), ('-0.6', '0.45'), ('0.15', '-0.8')]


def direct(c, limit):
    z = mp.mpc(0); d = mp.mpc(0)
    for n in range(1, limit + 1):
        d = 2*z*d + 1
        z = z*z + c
        if abs(z) > 2:
            return n, True, z, d
    return limit, False, z, d


def reference(c, limit):
    z = mp.mpc(0); ref = [0j]
    for _ in range(limit):
        z = z*z + c
        ref.append(complex(z))
        if abs(z) > 4:
            break
    return ref


def build(ref, rho, order, tol=1e-12):
    # b_k = a_k rho^k; x = dc/rho, |x| <= 1.
    b = [0j] * (order + 1)
    error = deriv_error = 0.0
    accepted = (0, b[:], error, deriv_error, 'limit')
    for n in range(len(ref)-1):
        p = sum(abs(v) for v in b)
        q = sum(k*abs(b[k]) for k in range(1, order+1))
        tail = tail_d = 0.0
        for a in range(1, order+1):
            for h in range(1, order+1):
                if a+h > order:
                    t = abs(b[a])*abs(b[h])
                    tail += t; tail_d += (a+h)*t
        next_error = 2*(abs(ref[n])+p)*error + error*error + tail
        next_deriv_error = (2*(abs(ref[n])+p+error)*deriv_error
                            + 2*error*q + tail_d)
        v = [0j] * (order+1)
        for k in range(1, order+1):
            v[k] = 2*ref[n]*b[k] + sum(b[h]*b[k-h] for h in range(1, k))
        v[1] += rho
        pnext = sum(abs(t) for t in v)
        qnext = sum(k*abs(v[k]) for k in range(1, order+1))
        if not all(math.isfinite(t) for t in (pnext, qnext, next_error, next_deriv_error)):
            reason = 'nonfinite'
        elif abs(ref[n+1])+pnext+next_error >= 2:
            reason = 'bailout-envelope'
        elif next_error > tol:
            reason = 'state-tail'
        elif next_deriv_error > 1e-8*max(rho, qnext):
            reason = 'derivative-tail'
        else:
            b = v; error = next_error; deriv_error = next_deriv_error
            accepted = (n+1, b[:], error, deriv_error, 'limit')
            continue
        return (*accepted[:4], reason)
    return accepted


def evaluate(ref, rho, x, init, limit, transport=False):
    n, b, error, deriv_error, _ = init
    u = 0j; q = 0j
    # Simultaneous Horner evaluation; q = du/dx = rho dz/dc.
    for v in reversed(b):
        q = q*x + u
        u = u*x + v
    steps = 0
    while True:
        z = ref[n]+u
        if abs(z) > 2 or n == limit:
            return n, abs(z)>2, z, q, steps, True, error, deriv_error
        if n+1 == len(ref):
            return n, False, z, q, steps, False, error, deriv_error
        if transport:
            deriv_error = 2*(abs(z)+error)*deriv_error + 2*error*abs(q)
            error = 2*abs(z)*error + error*error
        q = 2*z*q + rho
        u = u*(2*ref[n]+u) + rho*x
        n += 1; steps += 1


def write_csv(path, rows):
    with path.open('w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)


def freeze(path, limit, dps):
    rows = []
    with mp.workdps(dps):
        for name, re, im, width in SCENES:
            c = mp.mpc(re, im); rho = mp.mpf(width)
            for xr, xi in ([('0.4', '0'), ('0.8', '0'), ('-0.6', '0.45')] if name.startswith('cusp') else OFFSETS):
                n, escaped, z, d = direct(c + rho*mp.mpc(xr, xi), limit)
                rows.append(dict(scene=name, x=[xr, xi], n=n, escaped=escaped,
                                 z=[mp.nstr(z.real, 45), mp.nstr(z.imag, 45)],
                                 q=[mp.nstr((rho*d).real, 45), mp.nstr((rho*d).imag, 45)]))
            print('frozen', name, [r['n'] for r in rows[-3:]], flush=True)
    path.write_text(json.dumps(dict(limit=limit, dps=dps, scenes=SCENES, offsets=OFFSETS,
                                   rows=rows), indent=2)+'\n')


def probe(truth_path, output):
    started = time.perf_counter()
    truth_bytes = truth_path.read_bytes(); truth = json.loads(truth_bytes)
    assert truth['scenes'] == [list(s) for s in SCENES] and truth['offsets'] == [list(x) for x in OFFSETS]
    limit = truth['limit']; rows = []; summary = []
    with mp.workdps(truth['dps']):
        for name, re, im, width in SCENES:
            rho = float(width)
            t = time.perf_counter(); ref = reference(mp.mpc(re, im), limit)
            ref_s = time.perf_counter()-t
            expected = [r for r in truth['rows'] if r['scene']==name]
            xs = [complex(float(r['x'][0]), float(r['x'][1])) for r in expected]
            baseline = None
            for order, tol in [(0, 1e-12), (2, 1e-12), (4, 1e-12), (8, 1e-12), (4, 1e-18), (8, 1e-18)]:
                builds = []; times = []
                for _ in range(3):
                    t = time.perf_counter()
                    init = build(ref, rho, order, tol) if order else (0, [0j], 0., 0., 'plain')
                    builds.append(time.perf_counter()-t if order else 0.)
                    t = time.perf_counter()
                    results = [evaluate(ref, rho, x, init, limit) for x in xs]
                    times.append((time.perf_counter()-t)/len(xs))
                prep = statistics.median(builds); per_pixel = statistics.median(times)
                if not order: baseline = per_pixel
                results = [evaluate(ref, rho, x, init, limit, transport=True) for x in xs]
                errors = 0
                for target, x, result in zip(expected, xs, results):
                    n, escaped, z, q, steps, supported, end_error, end_deriv_error = result
                    same = supported and n==target['n'] and escaped==target['escaped']
                    ztruth = complex(*map(float, target['z']))
                    qtruth = complex(*map(float, target['q']))
                    ze = abs(z-ztruth) if same else math.nan
                    qe = abs(q-qtruth)/max(abs(qtruth), 1e-300) if same else math.nan
                    nuerr = abs(math.log2(math.log(abs(z)))-math.log2(math.log(abs(ztruth)))) if same and escaped else math.nan
                    correct = same and ze <= 1e-6*max(1, abs(ztruth)) and qe <= 1e-6 and (not escaped or nuerr <= 1e-6)
                    errors += not correct
                    rows.append(dict(scene=name, order=order, tolerance=tol, x=str(x), escape_truth=target['n'],
                        escaped_truth=target['escaped'], n=n, escaped=escaped, supported=supported,
                        correct=correct, state_abs_error=ze, derivative_relative_error=qe,
                        smooth_abs_error=nuerr, prefix=init[0], ordinary=steps,
                        end_truncation_bound=end_error, end_derivative_bound=end_deriv_error,
                        truncation_bound=init[2], normalized_derivative_bound=init[3], stop=init[4]))
                delta = baseline-per_pixel
                summary.append(dict(scene=name, order=order, tolerance=tol, errors=errors, pixels=len(xs),
                    prefix=init[0], stop=init[4], reference_s=ref_s, setup_s=prep,
                    pixel_s=per_pixel, break_even_pixels=math.ceil(prep/delta) if delta>0 else 'never',
                    batch_speedup=(ref_s+len(xs)*baseline)/(ref_s+prep+len(xs)*per_pixel),
                    projected_480x270_speedup=(ref_s+129600*baseline)/(ref_s+prep+129600*per_pixel),
                    score=0 if errors else (ref_s+len(xs)*baseline)/(ref_s+prep+len(xs)*per_pixel)))
            print(name, 'prefixes', [r['prefix'] for r in summary[-6:]],
                  'errors', [r['errors'] for r in summary[-6:]], flush=True)
    output.mkdir(parents=True, exist_ok=True)
    write_csv(output/'pixels.csv', rows); write_csv(output/'summary.csv', summary)
    (output/'run.json').write_text(json.dumps(dict(truth_sha256=hashlib.sha256(truth_bytes).hexdigest(),
        limit=limit, repeats=3, elapsed_s=time.perf_counter()-started, python=__import__('sys').version, mpmath=mp.__version__,
        note='Python sparse probe; projected frame timings are extrapolations, not renders'), indent=2)+'\n')
    print('rows', len(rows), 'accuracy failures', sum(not r['correct'] for r in rows))


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--freeze', action='store_true')
    p.add_argument('--truth', type=Path, default=HERE/'truth.json')
    p.add_argument('--output', type=Path, default=HERE/'results')
    p.add_argument('--limit', type=int, default=100000)
    p.add_argument('--dps', type=int, default=100)
    a = p.parse_args()
    if a.limit < 1 or a.dps < 60: p.error('limit >= 1 and dps >= 60 required')
    if a.freeze: freeze(a.truth, a.limit, a.dps)
    else: probe(a.truth, a.output)
