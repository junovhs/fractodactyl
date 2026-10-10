"""FACT-01: parameter-dependent M(24,2) ladder return, factored over its 2-cycle.

Usage: python tools/research/misiurewicz/factored_return.py --period 764|1582|16116|all
The chart guard is empirical, not a proof of a whole-frame error bound. Decline
outside it; a renderer must use direct iteration as its fallback (DEC-10/17).
"""
import argparse
import time
from pathlib import Path

from mpmath import mp

Q = 24
L = 2
TERMS = 48
CHART_RADIUS = mp.mpf('1e-3')
RE0 = '-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
IM0 = '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
OFFSETS = (('.49', '.27'), ('-.49', '.27'), ('.1', '.05'), ('-.2', '-.1'))
WIDTHS = (5, 5000)


def ladder_c(period):
    if period == 764:
        return mp.mpc(RE0, IM0)
    path = Path(__file__).with_name('ladder_rungs.txt')
    for line in path.read_text().splitlines():
        if line.startswith('#') or not line.strip():
            continue
        _, p, re, im = line.split()
        if int(p) == period:
            return mp.mpc(re, im)
    raise ValueError(f'no ladder nucleus for period {period}')


def precision(period):
    # Enough digits to resolve the parameter-width scale and verify derivatives.
    return {764: 135, 1582: 185, 16116: 1080}[period]


def scales(period, rho):
    k = (period - 764) // 2
    sc = mp.mpf('4.1205e-50') * abs(rho)**(-2*k)
    return sc, mp.sqrt(sc)


def chart(c, period):
    """Build K_c, dK_c/dc; no orbit work depending on P."""
    n = (period-Q-L)//2
    # Choose the 2-cycle point reached by f^24(c), not by f^24(0).
    root = mp.sqrt(-3-4*c)
    a0, a1 = (-1+root)/2, (-1-root)/2
    z = c
    for _ in range(Q):
        z = z*z+c
    a = min((a0, a1), key=lambda x: abs(z-x))
    b = -1-a
    rho = 4*(c+1)
    da = (2*b+1)/(1-rho)
    db = -da
    g2, g3 = 2*b+4*a*a, 4*a
    dg2, dg3 = 2*db+8*a*da, 4*da
    K = [mp.mpc(0) for _ in range(TERMS+1)]
    dK = K.copy()
    p2, p3, p4 = (K.copy() for _ in range(3))
    dp2, dp3, dp4 = (K.copy() for _ in range(3))
    K[1] = mp.mpc(1)
    min_pivot = mp.inf
    for j in range(2, TERMS+1):
        def product(x, y, dx, dy):
            s = ds = mp.mpc(0)
            for i in range(1, j):
                s += x[i]*y[j-i]
                ds += dx[i]*y[j-i]+x[i]*dy[j-i]
            return s, ds
        p2[j], dp2[j] = product(K, K, dK, dK)
        p3[j], dp3[j] = product(K, p2, dK, dp2)
        p4[j], dp4[j] = product(K, p3, dK, dp3)
        num = g2*p2[j]+g3*p3[j]+p4[j]
        dnum = dg2*p2[j]+g2*dp2[j]+dg3*p3[j]+g3*dp3[j]+dp4[j]
        denom = rho**j-rho
        min_pivot = min(min_pivot, abs(denom))
        K[j] = num/denom
        dK[j] = (dnum-4*(j*rho**(j-1)-1)*K[j])/denom
    # Keep the exit tail bounded as the ladder period grows.
    tail = min(250, n-190)
    return dict(c=c, period=period, n=n, jump=n-tail, tail=tail,
                a=a, da=da, rho=rho, K=K, dK=dK,
                min_pivot=min_pivot, scale=scales(period, rho))


def series(model, w):
    """Return K(w), K_w(w), K_c(w) by Horner."""
    val = deriv = param = mp.mpc(0)
    for k in range(TERMS, -1, -1):
        deriv = deriv*w+val
        val = val*w+model['K'][k]
        param = param*w+model['dK'][k]
    return val, deriv, param


def evaluate(model, zin, din):
    """One return with total parameter derivative (seed din = dz_in/dc)."""
    c = model['c']
    z, dz = zin, din
    for _ in range(Q):
        dz = 2*z*dz+1
        z = z*z+c
    h = z-model['a']
    dh = dz-model['da']
    w = h
    for _ in range(12):
        v, d, _ = series(model, w)
        step = (v-h)/d
        w -= step
        if abs(step) <= max(abs(w), mp.mpf('1e-1000'))*mp.eps*64:
            break
    v, d, dc = series(model, w)
    if abs(v-h) > max(abs(h), mp.mpf('1e-1000'))*mp.eps*1024:
        return None, 'inverse residual'
    dw = (dh-dc)/d
    power = model['rho']**model['jump']
    outw = power*w
    if max(abs(w), abs(outw)) > CHART_RADIUS:
        return None, 'chart radius'
    doutw = power*(dw+4*model['jump']*w/model['rho'])
    value, slope, dc = series(model, outw)
    z = model['a']+value
    dz = model['da']+slope*doutw+dc
    # Estimate chart truncation amplification; not a rigorous tail bound.
    remainder = sum(abs(model['K'][k]*outw**k) for k in range(TERMS-7, TERMS+1))
    growth = mp.mpf(1)
    for _ in range(2*model['tail']+L):
        growth *= abs(2*z)
        dz = 2*z*dz+1
        z = z*z+c
    estimated_error = remainder*growth/model['scale'][1]
    if estimated_error > mp.mpf('1e-4'):
        return None, 'truncation estimate'
    return (z, dz, estimated_error), None


def direct(c, z, dz, period):
    for _ in range(period):
        dz = 2*z*dz+1
        z = z*z+c
    return z, dz


def operator_bytes(model):
    # Portable decimal serialization: independent of CPython allocator details.
    values = [model[k] for k in ('c', 'a', 'da', 'rho')]
    values += model['K']+model['dK']
    return sum(len(mp.nstr(mp.re(v), mp.dps).encode())+
               len(mp.nstr(mp.im(v), mp.dps).encode())+2 for v in values)


def probe(period):
    mp.dps = precision(period)
    started = time.perf_counter()
    c = ladder_c(period)
    model = chart(c, period)
    cold_build = time.perf_counter()-started
    patch_build = 0.0
    sc, sz = model['scale']
    cases = 0
    valid = 0
    failed = {}
    state_err = param_err = mp.mpf(0)
    eval_seconds = 0.0
    direct_seconds = 0.0
    for width in WIDTHS:
        for re, im in OFFSETS:
            cc = c+sc*width*mp.mpc(re, im)
            # Parameter-dependent operator: rebuild coefficients at this actual
            # pixel parameter; chart work is independent of P, but not free.
            t = time.perf_counter()
            patch = chart(cc, period)
            patch_build += time.perf_counter()-t
            z, dz = cc, mp.mpc(1)
            for _ in range(2):
                cases += 1
                t = time.perf_counter()
                factored, why = evaluate(patch, z, dz)
                eval_seconds += time.perf_counter()-t
                t = time.perf_counter()
                exact_z, exact_d = direct(cc, z, dz, period)
                direct_seconds += time.perf_counter()-t
                if why is None:
                    valid += 1
                    state_err = max(state_err, abs(factored[0]-exact_z)/sz)
                    param_err = max(param_err, abs(factored[1]-exact_d)*sc/sz)
                else:
                    failed[why] = failed.get(why, 0)+1
                z, dz = exact_z, exact_d  # Real inputs of successive biseries loops.
    return dict(period=period, dps=mp.dps, build=cold_build, patches=patch_build, eval=eval_seconds,
                direct=direct_seconds, bytes=operator_bytes(model),
                coverage=f'{valid}/{cases}', state=state_err, dc=param_err,
                coef=max(abs(x) for x in model['K'][1:]),
                dcoef=max(abs(x) for x in model['dK'][1:]),
                min_pivot=model['min_pivot'], failures=failed,
                tail=2*model['tail']+L)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--period', choices=['764','1582','16116','all'], default='all')
    args = parser.parse_args()
    periods = (764,1582,16116) if args.period=='all' else (int(args.period),)
    print('P dps cold_s patches_s eval_s direct_s bytes chart_ok max_state/sz max_dzdc*sc/sz max_coef max_dcoef min_pivot tail_steps failures', flush=True)
    for p in periods:
        r = probe(p)
        fmt = lambda x: mp.nstr(x, 4)
        print(f"{p} {r['dps']} {r['build']:.3f} {r['patches']:.3f} {r['eval']:.3f} {r['direct']:.3f} {r['bytes']} {r['coverage']} {fmt(r['state'])} {fmt(r['dc'])} {fmt(r['coef'])} {fmt(r['dcoef'])} {fmt(r['min_pivot'])} {r['tail']} {r['failures']}", flush=True)
        if r['coverage'] != '16/16' or max(r['state'],r['dc']) > mp.mpf('1e-3'):
            raise SystemExit(f'FAIL P={p}: stop before testing deeper rungs')


if __name__ == '__main__':
    main()
