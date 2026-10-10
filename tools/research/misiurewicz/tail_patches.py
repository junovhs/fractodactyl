"""PROB-09 variant: an adaptive atlas of analytic patches for the post-jump tail.

After the Koenigs jump every pixel sits at z = A + psi(w), w on the fundamental ring
R0/|rho| <= |w| < R0, and the rest of its orbit is F_n(w) = f_C^n(A + psi(w)). F_n is
entire in w (unlike nu, which is fractal: PROB-04), so it can be stored exactly: cover
s = log w by squares, and on each square keep a degree-D Taylor polynomial of F_n for the
largest n at which the polynomial still matches F_n to TOL. Squares whose n falls well
short of their pixels' escape time are split. A pixel then costs one leaf lookup, one
polynomial and the remaining T(w) - n plain steps.

Reports leaves vs mean remaining tail steps and the max patch error on fresh samples.
Usage: python tail_patches.py [D=16] [TOL=1e-14] [MAXDEPTH=8] [GAIN=6] [abs|rel] [CONSTS_TO_APPEND]
"""
import sys, math, time
import numpy as np
D = int(sys.argv[1]) if len(sys.argv) > 1 else 16
TOL = float(sys.argv[2]) if len(sys.argv) > 2 else 1e-14
MAXDEPTH = int(sys.argv[3]) if len(sys.argv) > 3 else 8
GAIN = int(sys.argv[4]) if len(sys.argv) > 4 else 6
_mode = sys.argv[5] if len(sys.argv) > 5 else ''
_out = sys.argv[6] if len(sys.argv) > 6 else ''
centre = sys.argv[7:]
if len(centre) not in (0, 3):
    raise SystemExit("expected RE IM PERIOD together")
sys.argv = [sys.argv[0], 'unused']
import mpmath as mp
import koenigs_tail as kt, psi_series as ps
if centre:
    re, im, period = centre
    kt.DPS = max(kt.DPS, 80 + max(0, int(period) - 764) // 16)
    mp.mp.dps = kt.DPS
    kt.C = mp.mpc(re, im)
    kt.P = int(period)
# rel mode: TOL bounds the equivalent shift in s, |dz| <= TOL*|dF/ds| (a pixel error of
# |ds|/|ds/dpx|; measured |ds/dpx| >= ~1e-6 at 1080p on the six frames, so 1e-11 is 1e-5 px).
REL = _mode == 'rel'

R0 = 0.03; M = 64; ESC2 = 4.0; MAXN = 6000

def consts():
    mp.mp.dps = kt.DPS
    a, rho, coef = kt.setup()
    psi = ps.reverse(coef, 24)
    return complex(a), complex(rho), np.array([complex(c) for c in psi[1:]]), complex(kt.C)

A, RHO, PSI, C = consts()
S0 = math.log(R0/abs(RHO)); SL = math.log(abs(RHO))

def start(s):
    w = np.exp(s); h = np.zeros_like(w)
    for c in PSI[::-1]: h = (h + c)*w
    return A + h

def escape_steps(z, cap=MAXN):
    """steps until |z|^2 > ESC2 (vectorised)."""
    z = z.copy(); n = np.zeros(z.shape, int); live = np.ones(z.shape, bool)
    for _ in range(cap):
        if not live.any(): break
        z[live] = z[live]**2 + C; n[live] += 1
        live &= (z.real**2 + z.imag**2) <= ESC2
    return n

def fit(centers, r):
    """For squares (centre, half-width r): largest n whose degree-D Taylor of F_n in
    t = (s - centre)/r fits to TOL on the covering disk |t| <= sqrt2. Returns (n, coefs, minT)."""
    P = centers.size
    th = np.exp(2j*np.pi*np.arange(M)/M)
    ring = centers[:, None] + 1.5*r*th[None, :]                 # Cauchy circle, radius 1.5 r
    chk_t = np.concatenate([math.sqrt(2)*np.exp(2j*np.pi*(np.arange(12) + .5)/12), 0.7*th[::8], [0]])
    chk = centers[:, None] + r*chk_t[None, :]
    zr, zc = start(ring), start(chk)
    best_n = np.full(P, -1); best_c = np.zeros((P, D), complex); live = np.ones(P, bool)
    tpow = np.power.outer(chk_t, np.arange(D))                   # (K, D)
    for n in range(MAXN):
        if not live.any(): break
        cf = np.fft.fft(zr[live], axis=1)/M                        # coefficient k of (t/1.5)^k
        cf = cf[:, :D] / (1.5**np.arange(D))
        approx = cf @ tpow.T
        err = np.max(np.abs(approx - zc[live]), axis=1)
        big = np.max(np.abs(zr[live]), axis=1) > 2.0
        scale = np.abs(cf[:, 1])/r if REL else 1.0
        ok = (err <= TOL*scale) & ~big
        idx = np.nonzero(live)[0]
        best_n[idx[ok]] = n; best_c[idx[ok]] = cf[ok]
        live[idx[~ok]] = False
        zr[live] = zr[live]**2 + C; zc[live] = zc[live]**2 + C
    minT = escape_steps(start(chk)).min(axis=1)
    return best_n, best_c, minT

def build():
    """Quadtree over s. nodes[i] = [centre, r, first_child (-1 = leaf), leaf index]."""
    t = time.perf_counter()
    nx = max(1, round(2*math.pi/SL)); r0 = SL/2
    # root squares tile [S0, S0+SL) x [0, 2pi); r covers both half-sides (covering disks overlap)
    cy = (np.arange(nx) + .5)*(2*math.pi/nx); rr = max(r0, math.pi/nx)
    nodes = [[S0 + r0 + 1j*y, rr, -1, -1] for y in cy]
    level = (np.arange(nx), np.array(S0 + r0 + 1j*cy), rr, 0)
    leaves = []                                                   # (centre, r, depth, n, coefs)
    while level is not None:
        ids, cen, r, d = level
        n, cf, minT = fit(cen, r)
        split = (minT - n > GAIN) & (d < MAXDEPTH)
        for i in np.nonzero(~split)[0]:
            nodes[ids[i]][3] = len(leaves); leaves.append((cen[i], r, d, n[i], cf[i]))
        level = None
        if split.any():
            q = r/2; kid_ids = []; kid_cen = []
            for i in np.nonzero(split)[0]:
                nodes[ids[i]][2] = len(nodes)
                for sx in (-1, 1):                              # child order: (-,-), (-,+), (+,-), (+,+)
                    for sy in (-1, 1):
                        kid_ids.append(len(nodes)); kid_cen.append(cen[i] + q*(sx + 1j*sy))
                        nodes.append([cen[i] + q*(sx + 1j*sy), q, -1, -1])
            level = (np.array(kid_ids), np.array(kid_cen), q, d + 1)
    print(f"built {len(leaves)} leaves, {len(nodes)} nodes in {time.perf_counter() - t:.1f} s  (D {D}, TOL {TOL:g}, maxdepth {MAXDEPTH}, gain {GAIN})")
    return leaves, nodes, nx

def export(leaves, nodes, nx, path):
    """Append the tree to a koenigs_bench constants file (lines: patch_root, node, leaf)."""
    with open(path, 'a', encoding='utf-8') as f:
        f.write(f"patch_root {float(S0)!r} {float(SL)!r} {nx} {D}\n")
        for c, r, kid, li in nodes:
            f.write(f"node {float(c.real)!r} {float(c.imag)!r} {kid} {li}\n")
        for c, r, _, n, cf in leaves:
            f.write(f"leaf {float(c.real)!r} {float(c.imag)!r} {float(r)!r} {int(n)} "
                    + ' '.join(f"{float(x.real)!r} {float(x.imag)!r}" for x in cf) + "\n")

def evaluate(leaves, N=5000, seed=3):
    rng = np.random.default_rng(seed)
    s = S0 + SL*rng.random(N) + 2j*math.pi*rng.random(N)
    T = escape_steps(start(s))
    # leaf lookup by brute force over leaves (probe only)
    cen = np.array([l[0] for l in leaves]); rad = np.array([l[1] for l in leaves])
    rem = np.empty(N, int); err = np.empty(N)
    for k in range(N):
        inside = (np.abs((s[k] - cen).real) <= rad + 1e-15) & (np.abs((s[k] - cen).imag) <= rad + 1e-15)
        i = np.nonzero(inside)[0]
        i = i[np.argmax(rad[i] < np.inf)] if i.size else np.argmin(np.abs(s[k] - cen))
        c, r, _, n, cf = leaves[i]
        zt = start(np.array([s[k]]))[0]
        if n < 0: err[k] = 0; rem[k] = T[k]; continue      # invalid leaf: plain finish
        tt = (s[k] - c)/r
        z = np.polyval(cf[::-1], tt)
        for _ in range(n): zt = zt*zt + C
        err[k] = abs(z - zt); rem[k] = T[k] - n
    depth = np.bincount([l[2] for l in leaves])
    ns = np.array([l[3] for l in leaves])
    print(f"leaves per depth {depth.tolist()}  invalid {np.sum(ns < 0)}   leaf n: min {ns.min()} median {np.median(ns):.0f} max {ns.max()}")
    print(f"tail steps to |z|>2: before mean {T.mean():.1f} (p50 {np.median(T):.0f}, p95 {np.percentile(T,95):.0f})"
          f"  after mean {rem.mean():.1f} (p50 {np.median(rem):.0f}, p95 {np.percentile(rem,95):.0f})"
          f"   max patch err {err.max():.1e}")

if __name__ == '__main__':
    leaves, nodes, nx = build()
    if _out: export(leaves, nodes, nx, _out); print(f'appended tree to {_out}')
    evaluate(leaves)
