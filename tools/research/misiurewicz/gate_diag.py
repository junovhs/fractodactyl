"""PROB-09 step 1: where do the post-Koenigs-jump tail steps go?

After the jump every pixel sits at A + psi(w) with w in the fundamental ring
R0/|rho| <= |w| < R0 (Koenigs coordinate of the 2-cycle point A). So the whole tail is a
function on that ring. Sample it on a log-polar grid, iterate f_C in numpy, and for each
orbit count: total steps, steps within r of the alpha fixed point, the number of separate
visits to that disk (gate passes), and the longest single visit.
Usage: python gate_diag.py [TERMS=12] [R0=0.03] [N=400]
"""
import sys, math
import numpy as np
import mpmath as mp
TERMS = int(sys.argv[1]) if len(sys.argv) > 1 else 12
R0 = float(sys.argv[2]) if len(sys.argv) > 2 else 0.03
N = int(sys.argv[3]) if len(sys.argv) > 3 else 400
sys.argv = [sys.argv[0], 'return_map_truth.json', str(TERMS), str(R0)]
import koenigs_tail as kt

def consts():
    mp.mp.dps = kt.DPS
    a, rho, coef = kt.setup()
    alpha = (1 - mp.sqrt(1 - 4*kt.C))/2
    return complex(a), complex(rho), np.array([complex(c) for c in coef[1:TERMS+1]]), complex(alpha), complex(kt.C)

def psi(w, coef):
    h = w.copy()
    for _ in range(40):
        p = np.zeros_like(h); dp = np.zeros_like(h)
        for k in range(len(coef), 0, -1):
            dp = dp*h + p; p = p*h + coef[k-1]
        p = p*h
        st = (p - w)/dp; h -= st
        if np.max(np.abs(st)) < 1e-15: break
    return h

def run(A, rho, coef, alpha, C, rads=(0.05, 0.1, 0.2), maxit=20000):
    lr = np.linspace(math.log(R0/abs(rho)), math.log(R0), N, endpoint=False)
    th = np.linspace(0, 2*math.pi, N, endpoint=False)
    w = np.exp(lr[:, None] + 1j*th[None, :]).ravel()
    z = A + psi(w, coef)
    M = z.size; steps = np.zeros(M, int); alive = np.ones(M, bool)
    inside = {r: np.zeros(M, int) for r in rads}; visits = {r: np.zeros(M, int) for r in rads}
    run_ = {r: np.zeros(M, int) for r in rads}; best = {r: np.zeros(M, int) for r in rads}
    prev = {r: np.zeros(M, bool) for r in rads}
    for n in range(maxit):
        idx = np.nonzero(alive)[0]
        if idx.size == 0: break
        zz = z[idx]
        d = np.abs(zz - alpha)
        for r in rads:
            ins = d < r
            inside[r][idx] += ins
            visits[r][idx] += ins & ~prev[r][idx]
            run_[r][idx] = np.where(ins, run_[r][idx] + 1, 0)
            best[r][idx] = np.maximum(best[r][idx], run_[r][idx])
            prev[r][idx] = ins
        zz = zz*zz + C
        z[idx] = zz; steps[idx] += 1
        alive[idx] = (zz.real**2 + zz.imag**2) <= 1e20
    return w, steps, inside, visits, best

if __name__ == '__main__':
    A, rho, coef, alpha, C = consts()
    lam = 2*alpha
    print(f"A {A:.6f}  rho {rho:.6f} |rho| {abs(rho):.5f}  alpha {alpha:.6f}  lambda {lam:.6f} |lambda| {abs(lam):.5f}  |A-alpha| {abs(A-alpha):.4f}")
    w, steps, inside, visits, best = run(A, rho, coef, alpha, C)
    q = lambda a: ' '.join(f"{np.percentile(a, p):6.0f}" for p in (5, 25, 50, 75, 95, 99, 100))
    print(f"ring samples {w.size}  terms {TERMS}  R0 {R0}")
    print(f"tail steps           pct 5/25/50/75/95/99/max: {q(steps)}   mean {steps.mean():.1f}")
    for r in inside:
        print(f"r={r:4.2f} steps near alpha:                        {q(inside[r])}   mean {inside[r].mean():.1f}")
        print(f"       visits (passes):                           {q(visits[r])}")
        print(f"       longest visit:                             {q(best[r])}")
    hi = steps >= np.percentile(steps, 90)
    for r in inside:
        print(f"top-10% tails, r={r}: mean steps {steps[hi].mean():.0f}  near-alpha {inside[r][hi].mean():.0f}  visits {visits[r][hi].mean():.1f}  longest {best[r][hi].mean():.0f}")

