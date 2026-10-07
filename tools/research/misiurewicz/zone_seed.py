"""PROB-12 seed test: is the v0 zone (period-764 nucleus 1.7e-25 from M(24,2)) a lucky spot,
or do deep minibrots generally come with a short-cycle Misiurewicz structure?

For each centre: run the critical orbit, take the strongest record return |z_p| as the
minibrot period, Newton-refine the nucleus c_H, then rank Misiurewicz relations (q, r) at
c_H by D = |(z_{q+r} - z_q)/(u_{q+r} - u_q)| (auto-banding report), reduce to minimal q, and
report the eventual-cycle multiplier |lambda| = |prod 2 z| over one r-cycle (how fast the
Koenigs jump skips: ln|lambda| per r steps; near 1 is the slow, near-parabolic case).
Centres: the v0 centre, the classic valley zoom centre, and N random points near the
boundary of the set. Usage: python zone_seed.py [N=30] [SEED=1]
"""
import sys, random, time
import mpmath as mp

N = int(sys.argv[1]) if len(sys.argv) > 1 else 30
random.seed(int(sys.argv[2]) if len(sys.argv) > 2 else 1)
mp.mp.dps = 120
PMAX = 3000; RMAX = 8


def orbit(c, n):
    z = [mp.mpc(0)]; u = [mp.mpc(0)]
    for _ in range(n):
        u.append(2*z[-1]*u[-1] + 1); z.append(z[-1]**2 + c)
        if abs(z[-1]) > 1e6: break
    return z, u


def nucleus(c):
    """Atom-domain period: the n before escape (or within PMAX) with the smallest |z_n|; Newton
    to the period-p nucleus from c."""
    z, _ = orbit(c, PMAX)
    best, p = mp.inf, None
    for n in range(2, len(z)):
        if abs(z[n]) < best: best, p = abs(z[n]), n
    for _ in range(80):
        zz, uu = mp.mpc(0), mp.mpc(0)
        for _ in range(p): uu = 2*zz*uu + 1; zz = zz*zz + c
        st = zz/uu; c -= st
        if abs(st) < mp.mpf(10)**(-mp.mp.dps + 8): return c, p
    return None, None


def misiurewicz(c, p):
    z, u = orbit(c, p + RMAX + 2)
    best = (mp.inf, 0, 0)
    for r in range(1, RMAX + 1):
        for q in range(1, p//2):                         # q near p only echoes the p-periodic orbit
            dH = u[q + r] - u[q]
            if dH == 0: continue
            D = abs((z[q + r] - z[q])/dH)
            if D < best[0]*0.999: best = (D, q, r)
    D, q, r = best
    while q > 1:                                         # minimal preperiod with the same root
        dH = u[q - 1 + r] - u[q - 1]
        if dH == 0 or abs(abs((z[q - 1 + r] - z[q - 1])/dH)/D - 1) > 1e-3: break
        q -= 1
    lam = mp.mpf(1)
    for k in range(q, q + r): lam *= abs(2*z[k])
    return D, q, r, lam


def dwell(c, p, tol=1e-2):
    """Longest run of steps n < p where the nucleus orbit shadows an r-cycle (|z_{n+r} - z_n| <
    tol) that is repelling over the run (geometric mean of |2z| per step > 1): the steps a
    Koenigs jump at that cycle could skip. Returns (run, start, r, |lambda| per cycle)."""
    z, _ = orbit(c, p + RMAX + 1)
    best = (0, 0, 0, 0.0)
    for r in range(1, RMAX + 1):
        n = 1
        while n < p - r:
            if abs(z[n + r] - z[n]) < tol:
                m = n
                while m < p - r and abs(z[m + r] - z[m]) < tol: m += 1
                lam = 1.0
                for k in range(n, n + r): lam *= float(abs(2*z[k]))
                if m - n > best[0] and lam > 1.0: best = (m - n, n, r, lam)
                n = m
            n += 1
    return best


def dwell_any(c, p, tol=1e-2):
    """Like dwell, but over every cycle length r < p/2 (numpy on the double orbit): the
    longest run where the orbit repeats itself after r steps, repelling over the run."""
    import numpy as np
    z, _ = orbit(c, 2*p)
    zd = np.array([complex(x) for x in z[:p + p//2 + 1]])
    best = (0, 0, 0, 0.0)
    for r in range(1, p//2):
        close = np.abs(zd[r:p + r] - zd[:p]) < tol
        if not close.any(): continue
        # longest run of True
        d = np.diff(np.concatenate([[0], close.astype(int), [0]]))
        starts, ends = np.nonzero(d == 1)[0], np.nonzero(d == -1)[0]
        k = np.argmax(ends - starts); run, n0 = int(ends[k] - starts[k]), int(starts[k])
        if run <= best[0]: continue
        lam = float(np.prod(np.abs(2*zd[n0:n0 + r]))) if r < 400 else float('inf')
        if lam > 1.0: best = (run, n0, r, lam)
    return best


def main():
    centres = [('v0', mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502',
                              '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'))]
    centres.append(('valley', mp.mpc('-0.743643887037158704752191506114774', '0.131825904205311970493132056385139')))
    import numpy as np                                   # boundary points: escape after 500-3000 steps
    rng = np.random.default_rng(random.randrange(1 << 30)); found = []
    while len(found) < N:
        c = rng.uniform(-2, 0.5, 200000) + 1j*rng.uniform(-1.2, 1.2, 200000); z = np.zeros_like(c)
        it = np.zeros(c.size, int); live = np.ones(c.size, bool)
        for n in range(3000):
            z[live] = z[live]**2 + c[live]; it[live] += 1; live &= np.abs(z) < 2
        found += [complex(x) for x in c[(it > 500) & (it < 3000)]]
    for k, x in enumerate(found[:N]): centres.append((f'rnd{k + 1}', mp.mpc(x.real, x.imag)))
    print(f"{'name':8} {'p':>5} | {'dwell':>5} {'from':>5} {'r':>4} {'|lam|':>8} {'dwell/p':>7} | D-ranked: {'q':>4} {'r':>2} {'|lam|':>8}  secs")
    for name, c0 in centres:
        t = time.time()
        cH, p = nucleus(c0)
        if cH is None: print(f"{name:8} (no deep nucleus found)"); continue
        D, q, r, lam = misiurewicz(cH, p)
        z, u = orbit(cH, p + 1)
        size = 1/abs(u[p])**2                            # rough minibrot size ~ 1/|dz_p/dc|^2
        run, start, rr, lr = dwell_any(cH, p)
        print(f"{name:8} {p:5d} | {run:5d} {start:5d} {rr:4d} {lr:8.3g} {run/p:7.0%} | D-ranked: {q:4d} {r:2d} {mp.nstr(lam, 4):>8}  {time.time() - t:.1f}")


if __name__ == '__main__':
    main()
