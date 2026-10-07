"""Diagnostic: where do the post-Koenigs-jump tail steps go?

Reuses koenigs_tail.py's pipeline on the frozen 48-point truth. After the jump, every
direct step is labelled by the nearest landmark of f_C: the alpha fixed point, the
repelling 2-cycle points, or 'free'. Prints per-point step counts per label and the
multipliers, to tell whether a second analytic jump (another slow spiral) is available.
"""
import json, sys
from multiprocessing import Pool
import mpmath as mp
sys.argv = [sys.argv[0], sys.argv[1], '12', '0.03']
import koenigs_tail as kt

def landmarks():
    mp.mp.dps = kt.DPS
    C = kt.C
    s = mp.sqrt(1 - 4*C); fx = (1 - s)/2           # alpha fixed point (the one near -1/2)
    t = mp.sqrt(-3 - 4*C); c1 = (-1 + t)/2; c2 = (-1 - t)/2   # 2-cycle
    return dict(fixed=fx, cyc1=c1, cyc2=c2), 2*fx, 4*(1 + C)

def work(row):
    mp.mp.dps = kt.DPS
    if kt.A is None: kt.A, kt.RHO, kt.COEF = kt.setup()
    L, mfix, mcyc = landmarks()
    d = mp.mpc(*row['d']); c = kt.C + d
    z = c; k = 0
    while abs(z - kt.C) <= kt.GUARD and k < 60:
        for _ in range(kt.P): z = z*z + c
        k += 1
    for _ in range(23): z = z*z + kt.C
    h0 = z - kt.A
    if abs(h0) < kt.R0:
        w0 = kt.phi(h0)
        j = int(mp.floor(mp.log(kt.R0/abs(w0))/mp.log(abs(kt.RHO))))
        if j > 0: z = kt.A + kt.psi(w0*kt.RHO**j)
    counts = dict(fixed=0, cyc1=0, cyc2=0, free=0); trace = []; n = 0
    while abs(z) < 1e10 and n < 20000:
        dist = {name: abs(z - p) for name, p in L.items()}
        name = min(dist, key=dist.get)
        lab = name if dist[name] < 0.1 else 'free'
        counts[lab] += 1; trace.append((lab, float(dist[name])))
        z = z*z + kt.C; n += 1
    # longest run per label
    runs = {}; prev = None; r = 0
    for lab, _ in trace:
        r = r + 1 if lab == prev else 1; prev = lab
        runs[lab] = max(runs.get(lab, 0), r)
    return dict(w=row['w'], n=n, counts=counts, runs=runs,
                minfix=min(dd for lab, dd in trace if lab == 'fixed') if counts['fixed'] else None)

if __name__ == '__main__':
    rows = json.load(open(kt.TRUTH))['rows']
    _, mfix, mcyc = landmarks()
    print(f"multiplier fixed {mp.nstr(mfix,6)} |.|={mp.nstr(abs(mfix),6)}   2-cycle {mp.nstr(mcyc,6)} |.|={mp.nstr(abs(mcyc),6)}")
    with Pool(16) as pool: res = pool.map(work, rows)
    res.sort(key=lambda r: -r['n'])
    for r in res:
        print(f"w={float(r['w']):.0e} steps {r['n']:4d}  near-fixed {r['counts']['fixed']:4d} (run {r['runs'].get('fixed',0):3d})"
              f"  cyc {r['counts']['cyc1']+r['counts']['cyc2']:4d}  free {r['counts']['free']:4d}  min|z-fix| {r['minfix']}")
