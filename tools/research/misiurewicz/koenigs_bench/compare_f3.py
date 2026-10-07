"""BENC-04: compare fraktaler-3 EXR raw output with our pipeline at the same jittered points.

fraktaler-3 writes N = n + 1024 (all ones if unescaped) and NF = 1 - log2(ln|Z|^2/ln ER^2);
its continuous count N - 1024 + NF differs from ours (n + 1 - log2(log2|z|), same escape
radius 1e10) by a constant. That constant is removed as the median difference; the rest
is scored as px = |dnu| * ln2 * |DE| with fraktaler-3's own DE (pixels). fraktaler-3's
jitter frame index, y sign and row order are not documented, so every variant is tried
and the best kept.
Usage: python compare_f3.py OUT NXxNY WIDTH...
"""
import json, math, sys
import numpy as np


def read_exr(path):
    import OpenEXR
    if hasattr(OpenEXR, 'File'):
        with OpenEXR.File(path, separate_channels=True) as f:
            return {k: np.array(v.pixels) for k, v in f.channels().items()}
    import Imath
    f = OpenEXR.InputFile(path); dw = f.header()['dataWindow']
    w, h = dw.max.x - dw.min.x + 1, dw.max.y - dw.min.y + 1; out = {}
    for k, c in f.header()['channels'].items():
        t = c.type.v; dt = {0: np.uint32, 1: np.float16, 2: np.float32}[t]
        out[k] = np.frombuffer(f.channel(k, Imath.PixelType(t)), dtype=dt).reshape(h, w)
    return out


def main():
    out, size, widths = sys.argv[1], sys.argv[2], sys.argv[3:]
    nx, ny = map(int, size.split('x')); n = nx*ny
    ok = True
    for w in widths:
        ch = read_exr(f'{out}/f3-w{w}.exr')
        print('channels', sorted(ch), file=sys.stderr)
        N = ch.get('N', ch.get('N0')).astype(np.uint64).reshape(ny, nx)
        if 'N1' in ch:
            N = N | (ch['N1'].astype(np.uint64).reshape(ny, nx) << np.uint64(32))
        unesc = (N == np.uint64(0xFFFFFFFF)) | (N == np.uint64(0xFFFFFFFFFFFFFFFF))
        nu3 = N.astype(np.float64) - 1024 + ch['NF'].astype(np.float64).reshape(ny, nx)
        de = np.hypot(ch['DEX'].astype(np.float64), ch['DEY'].astype(np.float64)).reshape(ny, nx)
        best = None
        for fr in ('0', '-1'):
            for ys in ('1', '-1'):
                b = open(f'{out}/j{fr}{ys}/w{w}.bin', 'rb').read()
                kcl = np.frombuffer(b[:n], np.uint8).reshape(ny, nx)
                knu = np.frombuffer(b[n:n + 8*n], np.float64).reshape(ny, nx)
                for flip in (False, True):
                    kc, kn = (kcl[::-1], knu[::-1]) if flip else (kcl, knu)
                    both = (kc == 0) & ~unesc
                    if not both.any():
                        continue
                    d = nu3[both] - kn[both]; off = np.median(d)
                    px = np.abs(d - off)*math.log(2)*de[both]
                    mism = int(np.sum((kc == 0) != ~unesc))
                    score = (float(np.median(px)), mism)
                    if best is None or score < best[0]:
                        jj, ii = np.nonzero(both); worst = np.argsort(px)[::-1][:5]
                        worst = [dict(i=int(ii[k]), row=int(jj[k]), nu_f3=float(nu3[both][k] - off), nu_ours=float(kn[both][k]),
                                      de_px=float(de[both][k]), px=float(px[k])) for k in worst]
                        best = (score, dict(worst=worst, width=w, variant=f'frame {fr} ysign {ys} flip {flip}', offset=float(off),
                                            pixels=n, compared=int(both.sum()), class_mismatches=mism,
                                            wrong_px_gt_1e3=int(np.sum(px > 1e-3)), max_px=float(px.max()),
                                            p99_px=float(np.percentile(px, 99)), p50_px=float(np.median(px))))
        r = best[1]; ok &= r['wrong_px_gt_1e3'] == 0 and r['class_mismatches'] == 0
        print(json.dumps(r))
    print('AGREEMENT', 'PASS' if ok else 'FAIL')


if __name__ == '__main__':
    main()
