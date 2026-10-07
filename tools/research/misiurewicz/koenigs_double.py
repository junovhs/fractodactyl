"""Probe: run the whole exit tail (23 steps, Koenigs jump, finish) in IEEE double.

The loop prefix stays in mpmath (as in koenigs_tail.py). The exit point is passed as a
double offset delta = zeta - C from the critical orbit Z of f_C (zeta itself rounds to C
in double). The 23 approach steps are double perturbation against Z; the Koenigs input is
h0 = (Z_24 - alpha) + delta_24. phi coefficients are rounded to double, psi is Newton in
double, and the finish is plain double. Scored against the frozen 48-point truth with the usual px metric.
Usage: python koenigs_double.py TRUTH TERMS R0
"""
import json, math, sys, time
from multiprocessing import Pool
import mpmath as mp
import koenigs_tail as kt

def setup_double():
    a, rho, coef = kt.setup()
    return complex(a), complex(rho), [complex(c) for c in coef]

def tail_double(delta, Zd, Z24mA, Cd, A, RHO, COEF, R0):
    cf = COEF[1:]
    def phi(h):
        s = 0j
        for c in reversed(cf): s = (s + c)*h
        return s
    def dphi(h):
        s = 0j
        for k in range(len(cf), 0, -1): s = s*h + k*cf[k-1]
        return s
    steps = 0; jumped = 0
    for i in range(23): delta = 2*Zd[i]*delta + delta*delta; steps += 1
    h0 = Z24mA + delta; z = A + h0
    if abs(h0) < R0:
        w0 = phi(h0)
        j = int(math.floor(math.log(R0/abs(w0))/math.log(abs(RHO))))
        if j > 0:
            w = w0*RHO**j; h = w
            for _ in range(30):
                st = (phi(h) - w)/dphi(h); h -= st
                if abs(st) <= abs(h)*1e-16: break
            z = A + h; jumped = 2*j
    n = steps + jumped
    while True:
        z = z*z + Cd; n += 1; steps += 1
        a = z.real*z.real + z.imag*z.imag
        if a > 1e20: return n - math.log(math.log(math.sqrt(a)))/math.log(2), steps
        if steps > 20000: return None, steps

def work(args):
    row, terms, r0 = args
    mp.mp.dps = kt.DPS
    if kt.A is None: kt.A, kt.RHO, kt.COEF = kt.setup()
    d = mp.mpc(*row['d']); c = kt.C + d
    z = c; n = 1; k = 0
    while abs(z - kt.C) <= kt.GUARD and k < 60:
        for _ in range(kt.P): z = z*z + c; n += 1
        k += 1
    A, RHO, COEF = complex(kt.A), complex(kt.RHO), [complex(x) for x in kt.COEF[:terms+1]]
    Z = [kt.C]
    for _ in range(23): Z.append(Z[-1]**2 + kt.C)
    nu_rel, steps = tail_double(complex(z - kt.C), [complex(x) for x in Z], complex(Z[23] - kt.A),
                                complex(kt.C), A, RHO, COEF, r0)
    nu = n + nu_rel if nu_rel is not None else None
    de = float(mp.mpf(row['de'])); w = float(mp.mpf(row['w']))
    err = abs(nu - row['nu'])*math.log(2)*de/(w/480) if nu is not None else float('inf')
    return dict(w=row['w'], err=err, steps=steps)

if __name__ == '__main__':
    terms = int(sys.argv[2]); r0 = float(sys.argv[3])
    rows = json.load(open(kt.TRUTH))['rows']
    with Pool(16) as pool: res = pool.map(work, [(r, terms, r0) for r in rows])
    print(f"double tail  terms {terms}  r0 {r0}  points {len(res)}")
    for w in sorted({r['w'] for r in res}, key=lambda s: -float(s)):
        g = [r for r in res if r['w'] == w]
        print(f"w={float(w):.0e}: tail steps {min(r['steps'] for r in g)}-{max(r['steps'] for r in g)}  max px err {max(r['err'] for r in g):.1e}")
    print(f"overall max px err {max(r['err'] for r in res):.2e}")
