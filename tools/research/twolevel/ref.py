#!/usr/bin/env python3
"""Two-level reference probe (frontier question 4): exact centres and global reference orbits.

usage: ref.py LOC LOG10_WIDTH OUT.ref [NMAX]
LOC: eye (eye-of-universe centre), mis (Misiurewicz M(24,2) in seahorse valley, viewed
off-centre), nuc (lowest-period nucleus within W/2 of M(24,2)+0.3W, view centred on it).
Writes: int64 N, int32 K (pixel spacing h = 2^K for a 1920-wide frame), then N complex
doubles Z_0..Z_{N-1} (Z rounded from the high-precision orbit), plus OUT.txt metadata.
"""
import sys, struct, math
import gmpy2 as g
from gmpy2 import mpfr, mpc

HERE = __file__.rsplit('/', 1)[0]
CENSUS = HERE + '/../misiurewicz/census-locations.txt'

def census(name):
    for line in open(CENSUS):
        if line.startswith(name + '|'):
            f = line.strip().split('|')
            return f[1], f[2]
    raise KeyError(name)

def orbit(c, nmax, R2=1e10):
    z = mpc(0); zs = [complex(0)]
    for n in range(nmax):
        z = z * z + c
        zc = complex(z); zs.append(zc)
        if zc.real ** 2 + zc.imag ** 2 > R2:
            break
    return zs

def misiurewicz(c, pre, per, iters=60):
    # Newton on f(c) = z_{pre+per}(c) - z_{pre}(c)
    for _ in range(iters):
        z = mpc(0); dz = mpc(0); zp = dzp = None
        for n in range(1, pre + per + 1):
            dz = 2 * z * dz + 1; z = z * z + c
            if n == pre: zp, dzp = z, dz
        f = z - zp; df = dz - dzp
        step = f / df; c = c - step
        if abs(step) < mpfr(2) ** (-g.get_context().precision + 8):
            break
    return c

def box_period(c0, r, nmax):
    z = mpc(0); dz = mpc(0)
    for n in range(1, nmax + 1):
        dz = 2 * z * dz + 1; z = z * z + c0
        if abs(z) < r * abs(dz):
            return n
    return None

def nucleus(c, p, rmax, iters=200):
    # Newton on z_p(c) = 0, steps clamped to rmax (undamped Newton from the ball centre wanders)
    for _ in range(iters):
        z = mpc(0); dz = mpc(0)
        for _n in range(p):
            dz = 2 * z * dz + 1; z = z * z + c
        step = z / dz
        if abs(step) > rmax:
            step = step * (rmax / abs(step))
        c = c - step
        if abs(step) < mpfr(2) ** (-g.get_context().precision + 8):
            break
    return c

def nucleus_size(c, p):
    # standard size estimate (Heiland-Allen): |1 / (beta * lambda^2)|
    z = mpc(0); l = mpc(1); b = mpc(1)
    for _ in range(1, p):
        z = z * z + c; l = 2 * z * l; b = b + 1 / l
    return abs(1 / (b * l * l))

def centre_meta(loc, lw, nmax):
    W = 10.0 ** lw
    Wm = mpfr(10) ** int(lw) if lw == int(lw) else mpfr(W)
    meta = {}
    if loc == 'eye':
        re, im = census('eye-of-universe'); C = mpc(mpfr(re), mpfr(im))
    elif loc in ('mis', 'nuc'):
        re, im = census('v0')
        M = misiurewicz(mpc(mpfr('-0.74329189085243020293162432597251'), mpfr('0.13124055230879760477084590658147')), 24, 2)
        meta['M'] = str(M.real)[:40] + ' ' + str(M.imag)[:40]
        if loc == 'mis':
            C = M + mpc(mpfr('0.2317'), mpfr('-0.1381')) * Wm
        else:
            # nucleus of size ~W/30 near M: lowest-period nucleus within radius r of M+r*0.3,
            # with r chosen by bisection on log r so that size/W is close to 1/30
            best = None
            lo, hi = float(lw) * 0.5 - 1.0, float(lw) * 0.95
            for _ in range(14):
                mid = 0.5 * (lo + hi)
                r = mpfr(10) ** mpfr(mid)
                c0 = M + mpc(mpfr('0.3'), mpfr('0.17')) * r
                p = box_period(c0, r, nmax)
                if p is None:
                    hi = mid; continue
                Cn = nucleus(c0, p, r / 4)
                sz = float(nucleus_size(Cn, p) / Wm)
                if not sz > 0:
                    hi = mid; continue
                if best is None or abs(math.log10(sz * 30)) < abs(math.log10(best[2] * 30)):
                    best = (Cn, p, sz, mid)
                if sz > 1 / 30: lo = mid   # nucleus too big -> smaller radius
                else: hi = mid
                if abs(math.log10(sz * 30)) < 0.3: break
            C, p, sz, mid = best
            meta.update(period=p, size_over_W=sz, log10_r=mid)
    else:
        raise SystemExit('bad loc')
    return C, meta

def centre_of(loc, lw, nmax):
    return centre_meta(loc, lw, nmax)[0]

def main():
    loc, lw, out = sys.argv[1], float(sys.argv[2]), sys.argv[3]
    nmax = int(sys.argv[4]) if len(sys.argv) > 4 else 100000
    W = 10.0 ** lw
    K = math.floor(math.log2(W / 1920))
    bits = int(-lw * 3.33) + 160
    g.get_context().precision = bits
    C, meta = centre_meta(loc, lw, nmax)
    zs = orbit(C, nmax)
    with open(out, 'wb') as f:
        f.write(struct.pack('<qi', len(zs), K))
        for z in zs:
            f.write(struct.pack('<dd', z.real, z.imag))
    meta.update(loc=loc, log10w=lw, K=K, Nref=len(zs), bits=bits)
    with open(out + '.txt', 'w') as f:
        f.write(repr(meta) + '\n')
    print(meta)

if __name__ == '__main__':
    main()
