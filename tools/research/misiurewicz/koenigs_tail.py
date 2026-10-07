"""Probe: replace the exit tail's spiral-out along the repelling 2-cycle by a Koenigs jump.

Tail = orbit of exit point zeta under f_C(z) = z^2 + C (fixed parameter C, validated in PROB-07).
Candidate: iterate directly to land near the 2-cycle point alpha_C, linearise with the Koenigs
map phi (phi(g(h)) = rho*phi(h), g = f_C^2 around alpha), jump j cycle-steps analytically,
map back with psi = phi^-1, finish directly. No table, no interpolation.
"""
import json, math, sys
from multiprocessing import Pool
from pathlib import Path
import mpmath as mp

DPS = 80
mp.mp.dps = DPS
P = 764; GUARD = mp.mpf("1e-26"); MAXIT = 20000; R2 = mp.mpf(10)**20
TRUTH = Path(sys.argv[1])
C = mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502',
           '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')
NTERMS = int(sys.argv[2]) if len(sys.argv) > 2 else 40
R0 = mp.mpf(sys.argv[3]) if len(sys.argv) > 3 else mp.mpf('1e-3')

def setup():
    mp.mp.dps = DPS
    # alpha_C: the 2-cycle point the critical orbit of C lands near at step 24 (as for M(24,2)).
    z = mp.mpc(0)
    for _ in range(24): z = z*z + C
    a = z
    for _ in range(60):  # Newton on f^2(a) - a
        b = a*a + C; g = b*b + C - a; dg = 4*a*b - 1
        st = g/dg; a -= st
        if abs(st) < mp.mpf(10)**-(DPS-5): break
    b = a*a + C; rho = 4*a*b
    # g(h) = rho h + (2b + 4a^2) h^2 + 4a h^3 + h^4
    gc = [mp.mpc(0), rho, 2*b + 4*a*a, 4*a, mp.mpc(1)]
    N = NTERMS
    def mul(x, y):
        out = [mp.mpc(0)]*(N+1)
        for i, xi in enumerate(x):
            if xi == 0: continue
            for j, yj in enumerate(y):
                if i+j > N: break
                out[i+j] += xi*yj
        return out
    g = gc + [mp.mpc(0)]*(N+1-len(gc))
    powers = [None, g[:]]
    for k in range(2, N+1): powers.append(mul(powers[-1], g))
    coef = [mp.mpc(0)]*(N+1); coef[1] = mp.mpc(1)
    for n in range(2, N+1):
        s = sum(coef[k]*powers[k][n] for k in range(1, n))
        coef[n] = -s/(rho**n - rho)
    return a, rho, coef

A, RHO, COEF = None, None, None

def phi(h):
    return sum(c*h**k for k, c in enumerate(COEF) if k)

def dphi(h):
    return sum(k*c*h**(k-1) for k, c in enumerate(COEF) if k)

def psi(w):
    h = w
    for _ in range(50):
        st = (phi(h) - w)/dphi(h); h -= st
        if abs(st) <= abs(h)*mp.mpf(10)**-(DPS-10): break
    return h

def escape(z, n, c):
    for m in range(n, MAXIT):
        z = z*z + c
        a = z.real**2 + z.imag**2
        if a > R2:
            az = mp.sqrt(a); return float(m+1 - mp.log(mp.log(az))/mp.log(2)), m+1
    return None, MAXIT

def work(row):
    global A, RHO, COEF
    mp.mp.dps = DPS
    if A is None: A, RHO, COEF = setup()
    w = mp.mpf(row['w']); d = mp.mpc(*row['d']); c = C + d
    # exact prefix at the pixel parameter while the return input is within GUARD (PROB-07 schedule)
    z = c; n = 1; k = 0
    while abs(z - C) <= GUARD and k < 60:
        for _ in range(P):
            z = z*z + c; n += 1
            if abs(z) > 1e5: return dict(w=row['w'], skip='escaped-in-return')
        k += 1
    zeta, n0 = z, n
    direct, nd = escape(zeta, n0, C)               # fixed-C direct tail (PROB-07 control)
    # candidate: direct 23 steps to reach the alpha neighbourhood, Koenigs jump, finish directly
    z = zeta; m = n0
    for _ in range(23): z = z*z + C; m += 1
    h0 = z - A; used = 23; jumped = 0
    if abs(h0) < R0:
        w0 = phi(h0)
        j = int(math.floor(float(mp.log(R0/abs(w0))/mp.log(abs(RHO)))))
        if j > 0:
            hw = psi(w0*RHO**j); z = A + hw; m += 2*j; jumped = 2*j
    cand, nc = escape(z, m, C)
    used += (nc - m)
    truth = row['nu']; de = mp.mpf(row['de'])
    px = lambda v: abs(v - truth)*math.log(2)*float(de/(w/480))
    return dict(w=row['w'], k=k, tail_steps=nd - n0, cand_direct_steps=used, jumped=jumped,
                err_direct=px(direct + 0), err_cand=px(cand) if cand is not None else float('inf'),
                h0=float(abs(h0)))

if __name__ == '__main__':
    rows = json.load(open(TRUTH))['rows']
    with Pool(16) as pool: res = pool.map(work, rows)
    res = [r for r in res if 'skip' not in r]
    print(f"terms {NTERMS}  r0 {mp.nstr(R0,3)}  points {len(res)}")
    for w in sorted({r['w'] for r in res}, key=lambda s: -float(s)):
        g = [r for r in res if r['w'] == w]
        print(f"w={float(w):.0e}: n {len(g)}  tail steps {min(r['tail_steps'] for r in g)}-{max(r['tail_steps'] for r in g)}"
              f"  -> candidate direct steps {min(r['cand_direct_steps'] for r in g)}-{max(r['cand_direct_steps'] for r in g)}"
              f"  (jumped {min(r['jumped'] for r in g)}-{max(r['jumped'] for r in g)})"
              f"  | max px err: direct {max(r['err_direct'] for r in g):.1e}  KOENIGS {max(r['err_cand'] for r in g):.1e}")
