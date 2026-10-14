#!/usr/bin/env python3
"""Fallback floor: the 1% quantile of truth DE (px) per view versus the guard's sigma.
A guard needing lambda*sigma <= DE falls back on at least P(DE < lambda*sigma) of pixels,
whatever the arithmetic, so the sigma needed for <=1% fallback is q1%(DE)/lambda."""
import sys, glob, numpy as np
import analyze as A
S = sys.argv[1]
print('| view | escaped share with DE < 1e-4 px | DE 1% quantile (px) | median sigma T=8 (px) | sigma needed / sigma at T=8 | extra bits needed |')
print('|---|---|---|---|---|---|')
for w in ('-10', '-30', '-100', '-300'):
    for loc in ('eye', 'mis', 'nuc'):
        f = f'{S}/rec/{loc}{w}.T8'
        try: r = A.load(f)
        except Exception: continue
        esc = r['tn'] >= 0
        de = np.where(esc, r['tde'], 0)        # interior counts as DE 0 (always guarded)
        q1 = np.quantile(de, 0.01)
        sig = np.median(r['sig'][r['rn'] >= 0])
        need = q1 / 4
        ratio = need / sig
        print(f'| {loc}{w} | {100*np.mean(esc & (r["tde"] < 1e-4)):.2f}% | {q1:.1e} | {sig:.1e} | {ratio:.1e} | {max(0, -np.log2(ratio)):.0f} |')
