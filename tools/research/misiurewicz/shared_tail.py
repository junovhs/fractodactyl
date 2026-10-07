"""PROB-12 seed test: can shallower v0 bands reuse a fixed-parameter tail?

The fast path runs the exit tail (Koenigs jump + patch atlas + finish) at one fixed
parameter. In the deep band (1e-35..) pixels are within 1e-35 of it, so that is free. For a
shallower band, does swapping the pixel's true c for a shared parameter for the last T
steps before escape move the pixel by less than 1e-3 px? Shared parameters tried: the
band's own minibrot nucleus c_H and the deep period-764 nucleus C.
Per pixel: 80-digit truth (nu, DE); then the same orbit with c until n_esc - T, then the
shared parameter to escape. Error = |dnu| * ln2 * DE_px (fd convention, DE = 2|z|ln|z|/|dz/dc|).
Usage: python shared_tail.py [PIXELS=60]
"""
import sys, math, random, time
import mpmath as mp
mp.mp.dps = 80
NPIX = int(sys.argv[1]) if len(sys.argv) > 1 else 60
CEN = mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502',
             '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')
C764 = CEN
BANDS = [(mp.mpf('1e-9'), 241), (mp.mpf('1e-15'), 435)]
TS = (300, 600, 1000)


def nucleus(p):
    c = CEN
    for _ in range(80):
        z, u = mp.mpc(0), mp.mpc(0)
        for _ in range(p): u = 2*z*u + 1; z = z*z + c
        st = z/u; c -= st
        if abs(st) < mp.mpf(10)**-70: break
    return c


def run(c, switch=None, c2=None, maxit=20000, linear=False):
    """linear: after the switch, iterate at c2 and carry the first-order correction
    d' = 2 z d + (c - c2) (what a stored dF/dc patch would supply); escape uses z + d."""
    z, dz, d = mp.mpc(0), mp.mpc(0), mp.mpc(0)
    for n in range(1, maxit + 1):
        sw = switch is not None and n > switch
        cc = c2 if sw else c
        if sw and linear: d = 2*z*d + (c - c2)
        dz = 2*z*dz + 1; z = z*z + cc
        if linear and sw and abs(z + d) > 1e10:
            z = z + d; return n, n + 1 - float(mp.log(mp.log(abs(z), 2), 2)), None
        if abs(z) > 1e10:
            return n, n + 1 - float(mp.log(mp.log(abs(z), 2), 2)), float(2*abs(z)*mp.log(abs(z))/abs(dz))
    return None, None, None


def main():
    random.seed(1); t0 = time.time()
    print(f"{'width':>6} {'shared':>8} {'T':>4}  {'max px':>8} {'p90 px':>8} {'>1e-3':>6}  (pixels)")
    for W, p in BANDS:
        cH = nucleus(p); px = W/480
        pix = []
        while len(pix) < NPIX:
            c = CEN + mp.mpc(random.uniform(-.5, .5)*W, random.uniform(-.28, .28)*W)
            n, nu, de = run(c)
            if n: pix.append((c, n, nu, de/px))
        for label, c2, lin in (('C764', C764, False), ('C764+lin', C764, True)):
            for T in TS:
                errs = []
                for c, n, nu, de in pix:
                    _, nu2, _ = run(c, max(0, n - T), c2, linear=lin)
                    errs.append(abs(nu2 - nu)*math.log(2)*de if nu2 is not None else float('inf'))
                errs.sort()
                print(f"{mp.nstr(W, 1):>6} {label:>8} {T:4d}  {errs[-1]:8.1e} {errs[int(.9*len(errs))]:8.1e} {sum(e > 1e-3 for e in errs):6d}  ({len(errs)})")
    print(f"({time.time() - t0:.0f} s)")


if __name__ == '__main__':
    main()
