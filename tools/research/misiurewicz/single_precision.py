"""BENC-08 seed test: can the deep pipeline run in single precision (GPU speed)?

Reimplements the koenigs_bench pixel pipeline (loops, 23 approach steps, phi, Koenigs jump,
tail-patch lookup, finish) in numpy over whole 480x270 frames, with each stage's precision
switchable, and scores every pixel against fd per-frame BLA (|dnu|*ln2*de, bar 1e-3 px).
Variants:
  A  all float64 (sanity: should match the Rust bench)
  B  all float32 (a naive GPU port)
  F  float32 loops/approach/jump/patch, float64 finish (is the front end OK in float32?)
  C  float32 everywhere, but the finish iterates a float32 offset from a float64 reference
     orbit per tail patch (the patch centre's continued orbit), as GPU renderers do
Usage: python single_precision.py CONSTS FD_REF_DIR [WIDTHS...]
"""
import math, os, sys, time
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'koenigs_bench'))
import compare

NX, NY = 480, 270


def load(path):
    k = dict(orbit=[], bis=[], phi=[], leaves=[], nodes=[])
    for line in open(path):
        f = line.split()
        if not f or f[0].startswith('#'): continue
        cx = lambda i: complex(float(f[i]), float(f[i + 1]))
        t = f[0]
        if t in ('c', 'bias', 'alpha', 'rho', 'z24_minus_alpha'): k[t] = cx(1)
        elif t in ('scale', 'guard', 'r0', 'period'): k[t] = float(f[1])
        elif t == 'orbit': k['orbit'].append(cx(2))
        elif t == 'biseries': k['bis'].append((int(f[1]), int(f[2]), cx(3)))
        elif t == 'phi': k['phi'].append(cx(2))
        elif t == 'patch_root': k['nx'] = int(f[3])
        elif t == 'node': k['nodes'].append((cx(1), int(f[3]), int(f[4])))
        elif t == 'leaf': k['leaves'].append((cx(1), float(f[3]), int(f[4]), [cx(5 + 2*i) for i in range((len(f) - 5)//2)]))
    k['deg'] = max(i + j for i, j, _ in k['bis'])
    nd = k['nodes']
    k['ncen'] = np.array([c for c, _, _ in nd]); k['nkid'] = np.array([a for _, a, _ in nd]); k['nleaf'] = np.array([b for _, _, b in nd])
    lv = k['leaves']
    k['lcen'] = np.array([c for c, _, _, _ in lv]); k['lr'] = np.array([r for _, r, _, _ in lv])
    k['ln'] = np.array([m for _, _, m, _ in lv]); k['lcf'] = np.array([cf for _, _, _, cf in lv])
    return k


def frame(k, W, prec):
    """prec: dict stage -> dtype (complex64/complex128) for 'front' and 'finish'; prec['ref']
    True for the per-patch reference-orbit finish (variant C)."""
    fr, fi = prec['front'], prec['finish']
    rf = np.float32 if fr == np.complex64 else np.float64
    S = k['scale']; h = W/NX
    i, j = np.meshgrid(np.arange(NX), np.arange(NY))
    v = (((i + 0.5 - NX/2)*h) + 1j*(-(j + 0.5 - NY/2)*h)).ravel()/S
    lp = prec.get('loop', fr); rl = np.float32 if lp == np.complex64 else np.float64
    v = v.astype(lp); M = v.size
    # loops: u <- sum_i b_i(v) u^i while |u| S <= guard
    deg = k['deg']; b = [np.zeros(M, lp) for _ in range(deg + 1)]
    for a, bb, cf in k['bis']: b[a] += lp(cf)*v**bb
    b[0] += lp(k['bias'])
    u = v.copy(); n = np.ones(M, np.int64); act = np.abs(u)*S <= k['guard']
    for _ in range(60):
        if not act.any(): break
        s = b[deg][act]
        for a in range(deg - 1, -1, -1): s = s*u[act] + b[a][act]
        u[act] = s; n[act] += int(k['period'])
        act[act] = np.abs(s)*S <= k['guard']
    # approach
    d = (u*rl(S)).astype(lp)
    for z in k['orbit']: d = lp(2*z)*d + d*d
    n += 23
    gl = prec.get('glue', fr); rg = np.float32 if gl == np.complex64 else np.float64
    h0 = gl(k['z24_minus_alpha']) + d.astype(gl)
    z = (gl(k['alpha']) + h0).astype(fi)
    jump = np.abs(h0) < k['r0']
    w0 = np.zeros(M, gl)
    for c in reversed(k['phi']): w0 = (w0 + gl(c))*h0
    lr = rg(math.log(abs(k['rho']))); ar = rg(math.atan2(k['rho'].imag, k['rho'].real))
    jj = np.floor(np.log(rg(k['r0'])/np.abs(w0))/lr)
    ok = jump & (jj >= 0)
    re = np.log(np.abs(w0)) + jj*lr
    im = np.mod(np.angle(w0).astype(rg) + jj*ar, rg(2*math.pi))
    node = np.minimum((im/(2*math.pi/k['nx'])).astype(int), k['nx'] - 1)
    for _ in range(40):
        kid = k['nkid'][node]; inner = kid >= 0
        if not inner.any(): break
        cen = k['ncen'][node[inner]]
        node[inner] = kid[inner] + 2*(re[inner] >= cen.real) + (im[inner] >= cen.imag)
    leaf = k['nleaf'][node]
    t = ((re - k['lcen'][leaf].real)/k['lr'][leaf] + 1j*(im - k['lcen'][leaf].imag)/k['lr'][leaf]).astype(fr)
    cf = k['lcf'][leaf].astype(fr)
    zp = np.zeros(M, fr); dp = np.zeros(M, fr)
    for q in range(cf.shape[1] - 1, -1, -1):
        zp = zp*t + cf[:, q]
        if q >= 1: dp = dp*t + cf[:, q]
    dp = dp*t                                         # offset from the patch centre value cf0
    z[ok] = zp[ok].astype(fi); n[ok] += (2*jj[ok]).astype(np.int64) + k['ln'][leaf[ok]]
    C = fi(k['c'])
    nu = np.full(M, np.nan); live = np.ones(M, bool)
    if prec.get('ref'):
        # per-leaf reference orbit (float64) from the patch centre value; pixels carry float32 offsets
        used = np.unique(leaf[ok]); refs = {}
        for L in used:
            zr = [complex(k['lcf'][L][0])]
            while abs(zr[-1]) < 1e10 and len(zr) < 20000: zr.append(zr[-1]**2 + k['c'])
            refs[L] = np.array(zr)
        Lmax = max(len(r) for r in refs.values())
        R = np.full((k['lcf'].shape[0], Lmax), np.nan + 0j)
        for L, r in refs.items(): R[L, :len(r)] = r
        dlt = dp.astype(np.complex64); idx = np.where(ok, leaf, 0); step = np.zeros(M, int)
        refmode = ok.copy()
        for it in range(20000):
            if not live.any(): break
            # reference-offset pixels
            rm = live & refmode
            if rm.any():
                zr = R[idx[rm], step[rm]]
                dd = dlt[rm]
                dd = (np.complex64(2)*zr.astype(np.complex64))*dd + dd*dd
                step[rm] += 1; n[rm] += 1
                zr2 = R[idx[rm], np.minimum(step[rm], Lmax - 1)]
                full = (zr2 + dd.astype(np.complex128))
                dlt[rm] = dd
                esc = np.abs(full) > 1e10
                ii = np.nonzero(rm)[0]
                nu[ii[esc]] = n[ii[esc]] + 1 - np.log2(np.log2(np.abs(full[esc])))
                live[ii[esc]] = False
                ended = ~esc & (np.isnan(zr2) | (step[rm] >= Lmax - 1) | (np.abs(full) > 2))
                z[ii[ended]] = full[ended].astype(fi); refmode[ii[ended]] = False
            pm = live & ~refmode
            if pm.any():
                zz = z[pm]*z[pm] + C; z[pm] = zz; n[pm] += 1
                esc = np.abs(zz.astype(np.complex128)) > 1e10
                ii = np.nonzero(pm)[0]
                nu[ii[esc]] = n[ii[esc]] + 1 - np.log2(np.log2(np.abs(zz[esc].astype(np.complex128))))
                live[ii[esc]] = False
    else:
        for it in range(20000):
            if not live.any(): break
            ii = np.nonzero(live)[0]
            zz = z[ii]*z[ii] + C; z[ii] = zz; n[ii] += 1
            a = np.abs(zz.astype(np.complex128))
            esc = a > 1e10
            nu[ii[esc]] = n[ii[esc]] + 1 - np.log2(np.log2(a[esc]))
            live[ii[esc]] = False
    return nu


def main():
    k = load(sys.argv[1]); fdref = sys.argv[2]
    widths = sys.argv[3:] or ['1e-35', '1e-38', '1e-40', '1e-43', '1e-46', '2e-48']
    all_w = ['1e-35', '1e-38', '1e-40', '1e-43', '1e-46', '2e-48']
    V = {'R f64 loops+glue, f32 patch, f32 offset-from-ref finish': dict(front=np.complex64, finish=np.complex64, glue=np.complex128, loop=np.complex128, ref=True),
         'P f64 all but f32 finish': dict(front=np.complex128, finish=np.complex64),
         'L f64 loops+approach+glue, f32 patch+finish': dict(front=np.complex64, finish=np.complex64, glue=np.complex128, loop=np.complex128),
         'G f32 + f64 glue (h0,phi,log,t)': dict(front=np.complex64, finish=np.complex64, glue=np.complex128),
         'H f32 + f64 glue, f64 finish': dict(front=np.complex64, finish=np.complex128, glue=np.complex128),
         'A all f64': dict(front=np.complex128, finish=np.complex128),
         'B all f32': dict(front=np.complex64, finish=np.complex64),
         'F f32 front, f64 finish': dict(front=np.complex64, finish=np.complex128),
         'C f32 + per-patch ref finish': dict(front=np.complex64, finish=np.complex64, ref=True)}
    print(f"{'variant':30} {'width':>6}  {'wrong>1e-3':>10} {'class mism':>10} {'max px':>9} {'p99 px':>9} {'p50 px':>9}  secs")
    for name, prec in V.items():
        for w in widths:
            t0 = time.time()
            cls, nufd, de = compare.load_fd(os.path.join(fdref, f'frame-{all_w.index(w):05d}.fds'), NX*NY)
            nu = frame(k, float(w), prec)
            fesc = (np.frombuffer(cls, np.uint8) & 3) == 0
            kesc = ~np.isnan(nu)
            both = fesc & kesc
            px = np.abs(nu[both] - np.array(nufd)[both])*math.log(2)*np.array(de)[both]
            print(f"{name:30} {w:>6}  {int((px > 1e-3).sum()):10d} {int((fesc != kesc).sum()):10d} {px.max():9.1e} "
                  f"{np.percentile(px, 99):9.1e} {np.median(px):9.1e}  {time.time() - t0:.1f}")


if __name__ == '__main__':
    main()
