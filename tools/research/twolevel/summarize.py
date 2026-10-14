#!/usr/bin/env python3
"""Tables for the report. usage: summarize.py SCRATCH  (reads SCRATCH/rec/*.T*, SCRATCH/matrix*.log)"""
import sys, re, glob, numpy as np
import analyze as A, project as P

S = sys.argv[1]
logs = ''.join(open(f).read() for f in sorted(glob.glob(S + '/matrix*.log')))
def tileref(v, T):
    m = re.findall(rf'^{re.escape(v)} tile {T} .*?tileref iters/px ([0-9.]+)', logs, re.M)
    return float(m[-1]) if m else float('nan')

print('| view | T | p50 px | p99.99 px | max px (no guard) | nu fallback | nu cost share | kept max px | kept bad (px/cls) | full fallback | proj f32 ms | proj f64 ms | speed-up nu / full |')
print('|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for w in ('-10', '-30', '-100', '-300'):
    for loc in ('eye', 'mis', 'nuc'):
        v = loc + w
        for T in (4, 8, 16, 64, 2048):
            f = f'{S}/rec/{v}.T{T}'
            try:
                r = A.load(f)
            except Exception:
                continue
            both = (r['tn'] >= 0) & (r['rn'] >= 0)
            d = np.where(both, np.abs(r['rnu'] - r['tnu']) * A.LN2 * r['tde'], 0)[both]
            q = np.quantile(d, [0.5, 0.9999]); mx = d.max()
            tr = tileref(v, T)
            fbn = P.rule_nu(r); fbf = P.rule_full(r)
            fn, wn, t32n, t64, sn = P.project(f, T, P.rule_nu, tr)
            ff, wf, t32f, _, sf = P.project(f, T, P.rule_full, tr)
            dd = np.where(both, np.abs(r['rnu'] - r['tnu']) * A.LN2 * r['tde'], 0)
            kept = ~fbn
            cls = int((((r['tn'] >= 0) != (r['rn'] >= 0)) & kept).sum())
            print(f'| {v} | {T} | {q[0]:.1e} | {q[1]:.1e} | {mx:.1e} | {100*fn:.2f}% | {100*wn:.0f}% | {(dd*kept).max():.1e} | {int(((dd>1e-3)&kept).sum())}/{cls} | {100*ff:.2f}% | {t32n:.1f} | {t64:.1f} | {sn:.1f}x / {sf:.1f}x |')
