"""Probe: the whole deep pixel in IEEE double, with no multiprecision per pixel.

Per pixel: v = (c - C)/SCALE in double; PROB-07 degree-D biseries loops in double while
|u|*SCALE <= GUARD; exit offset delta = SCALE*u; then koenigs_double's tail (perturbation
approach, Koenigs jump, plain-double finish). Per-path constants (biseries coefficients,
nucleus residual, critical orbit, alpha, phi) are computed once at high precision and
rounded. Scored against the frozen 48-point truth.
Usage: python koenigs_double_full.py TRUTH DEGREE TERMS R0 [GUARD=1e-26]
"""
import json, math, sys
import mpmath as mp
truth_path, DEG, TERMS, R0 = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), float(sys.argv[4])
GUARD = float(sys.argv[5]) if len(sys.argv) > 5 else 1e-26
sys.argv = [sys.argv[0], truth_path]
import koenigs_tail as kt, koenigs_double as kd, returns_exit_tail as rt

def main():
    terms, bias, build_s = rt.build_map(DEG)
    coeff = [(i, j, complex(rt.decode(a))) for i, j, a in terms]
    bias = complex(rt.decode(bias)); S = float(rt.SCALE)
    mp.mp.dps = kt.DPS
    A, RHO, COEF = kt.setup()
    Z = [kt.C]
    for _ in range(23): Z.append(Z[-1]**2 + kt.C)
    Zd = [complex(x) for x in Z]; Z24mA = complex(Z[23] - A)
    Ad, RHOd, COEFd = complex(A), complex(RHO), [complex(x) for x in COEF[:TERMS+1]]
    rows = json.load(open(truth_path))['rows']
    res = []
    for row in rows:
        v = complex(mp.mpc(*row['d'])/rt.SCALE)          # the pixel offset, rounded once
        vp = [v**j for j in range(DEG+1)]; u = v; k = 0
        while abs(u)*S <= GUARD and k < 60:
            up = [u**i for i in range(DEG+1)]
            u = bias + sum(a*up[i]*vp[j] for i, j, a in coeff); k += 1
        n = 1 + k*kt.P
        nu_rel, steps = kd.tail_double(S*u, Zd, Z24mA, complex(kt.C), Ad, RHOd, COEFd, R0)
        nu = n + nu_rel if nu_rel is not None else None
        de = float(mp.mpf(row['de'])); w = float(mp.mpf(row['w']))
        err = abs(nu - row['nu'])*math.log(2)*de/(w/480) if nu is not None else float('inf')
        res.append(dict(w=row['w'], err=err, k=k, steps=steps))
    print(f"all-double pixel  guard {GUARD:g}  degree {DEG}  terms {TERMS}  r0 {R0}  points {len(res)}  ({len(coeff)} biseries terms)")
    for w in sorted({r['w'] for r in res}, key=lambda s: -float(s)):
        g = [r for r in res if r['w'] == w]
        print(f"w={float(w):.0e}: loops {min(r['k'] for r in g)}-{max(r['k'] for r in g)}  tail steps "
              f"{min(r['steps'] for r in g)}-{max(r['steps'] for r in g)}  max px err {max(r['err'] for r in g):.1e}")
    print(f"overall max px err {max(r['err'] for r in res):.2e}  wrong(>1e-3) {sum(r['err'] > 1e-3 for r in res)}")

if __name__ == '__main__':
    main()
