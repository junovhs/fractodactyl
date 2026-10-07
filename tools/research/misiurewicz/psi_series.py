"""PROB-09: psi = phi^-1 by series reversion, so the Koenigs jump's inverse is one Horner
evaluation instead of Newton (mean 8.6, worst 30 iterations of two Horners each).
reverse(coef, N) returns psi_1..psi_N from phi_1..phi_N (phi_1 = 1).
Run directly: error of the N-term psi against exact (40-term phi, mpmath Newton) on the
fundamental ring R0/|rho| <= |w| <= R0.  Usage: python psi_series.py [R0=0.03]
"""
import sys, math, random
import mpmath as mp

def reverse(coef, N):
    """coef[k] = phi_k (coef[0] unused). Solve phi(psi(w)) = w term by term."""
    psi = [mp.mpc(0), mp.mpc(1)] + [mp.mpc(0)]*(N-1)
    def mul(x, y):
        out = [mp.mpc(0)]*(N+1)
        for i, xi in enumerate(x):
            if xi == 0: continue
            for j in range(N+1-i):
                if y[j] != 0: out[i+j] += xi*y[j]
        return out
    for n in range(2, N+1):
        # coefficient of w^n in sum_k phi_k psi^k with psi known up to n-1 (psi_n enters only via k=1)
        p = psi[:]; p[n] = mp.mpc(0); acc = mp.mpc(0); pw = p[:]
        for k in range(1, n+1):
            if k > 1: pw = mul(pw, p)
            if k < len(coef): acc += coef[k]*pw[n]
        psi[n] = -acc
    return psi

if __name__ == '__main__':
    R0 = float(sys.argv[1]) if len(sys.argv) > 1 else 0.03
    sys.argv = [sys.argv[0], 'unused']
    import koenigs_tail as kt
    mp.mp.dps = kt.DPS
    kt.A, kt.RHO, kt.COEF = kt.setup()
    PS = reverse(kt.COEF, 40)
    random.seed(1); ws = []
    for _ in range(200):
        r = math.exp(random.uniform(math.log(R0/abs(complex(kt.RHO))), math.log(R0)))
        t = random.uniform(0, 2*math.pi); ws.append(mp.mpc(r*math.cos(t), r*math.sin(t)))
    exact = [kt.psi(w) for w in ws]
    print("psi |coef| (every 4th):", ' '.join(mp.nstr(abs(PS[k]), 2) for k in range(1, 41, 4)))
    for N in (8, 10, 12, 14, 16, 20, 24):
        err = 0
        for w, e in zip(ws, exact):
            s = mp.mpc(0)
            for c in reversed(PS[1:N+1]): s = (s + c)*w
            err = max(err, abs(s - e)/abs(e))
        print(f"R0 {R0}  psi terms {N:2d}: max rel err {mp.nstr(err, 3)}")
