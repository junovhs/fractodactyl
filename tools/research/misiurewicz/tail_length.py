"""PROB-12 seed test (double precision, seconds): how long is the post-loop tail in a v0 band?

Perturbation in double against the band's minibrot nucleus (reference orbit computed once at
high precision; Zhuoran rebasing), vectorised over random pixels of a 480-px frame. A "loop"
is a close return |z| < EPS (the orbit passing the minibrot's centre); the tail is the steps
from the last loop to escape (|z| > 1e10). Compare with the reach of a shared fixed-parameter
tail (about 300 steps with first-order correction at 1e-15, shared_tail.py).
Usage: python tail_length.py WIDTH PERIOD [PIXELS=4000] [EPS=1e-3]
"""
import sys, time
import numpy as np
import mpmath as mp
W, P = float(sys.argv[1]), int(sys.argv[2])
NPIX = int(sys.argv[3]) if len(sys.argv) > 3 else 4000
EPS = float(sys.argv[4]) if len(sys.argv) > 4 else 1e-3
mp.mp.dps = 80
CEN = mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502',
             '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')


def main():
    t0 = time.time(); c = CEN
    for _ in range(80):
        z, u = mp.mpc(0), mp.mpc(0)
        for _ in range(P): u = 2*z*u + 1; z = z*z + c
        st = z/u; c -= st
        if abs(st) < mp.mpf(10)**-70: break
    Z = [mp.mpc(0)]
    for _ in range(P): Z.append(Z[-1]**2 + c)
    Zd = np.array([complex(x) for x in Z[:P]])              # periodic reference z_0..z_{P-1}
    rng = np.random.default_rng(1)
    off = complex(CEN - c)
    dc = off + (rng.uniform(-.5, .5, NPIX) + 1j*rng.uniform(-.28, .28, NPIX))*W
    d = np.zeros(NPIX, complex); m = np.zeros(NPIX, int)
    last = np.zeros(NPIX, int); esc = np.zeros(NPIX, int); live = np.ones(NPIX, bool)
    for n in range(1, 20001):
        i = np.nonzero(live)[0]
        if i.size == 0: break
        dd = 2*Zd[m[i]]*d[i] + d[i]**2 + dc[i]; mm = (m[i] + 1) % P
        zf = Zd[mm] + dd
        a = np.abs(zf)
        last[i[a < EPS]] = n
        out = a > 1e10; esc[i[out]] = n; live[i[out]] = False
        reb = a < np.abs(dd)                                  # rebase to the start of the reference
        dd[reb] = zf[reb]; mm[reb] = 0
        d[i] = dd; m[i] = mm
    e = esc > 0; tail = (esc - last)[e]
    q = lambda p: int(np.percentile(tail, p))
    print(f"width {W:g}, period {P}: {e.sum()} of {NPIX} escaped; escape step median {int(np.median(esc[e]))}")
    print(f"tail after last loop (|z|<{EPS:g}): p10 {q(10)}  p50 {q(50)}  p90 {q(90)}  p99 {q(99)}  max {tail.max()}"
          f"   share <= 300: {np.mean(tail <= 300):.0%}   ({time.time() - t0:.1f} s)")


if __name__ == '__main__':
    main()
