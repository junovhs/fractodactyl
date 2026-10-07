"""BENC-04: settle fraktaler-3 vs our pipeline disagreements with 90-digit direct iteration.

For the worst points listed by compare_f3.py (EXR column i, row), rebuild the bench's
sample of fraktaler-3's jittered point (frame 0, rows stored bottom-up: our internal row =
NY-1-row), iterate z^2 + c directly at 120 digits to |z| > 1e10, and print the true DE,
the nu error of both renderers (fraktaler-3's nu already shifted to our convention) and
the resulting pixel displacement with the true DE.
Usage: python truth_f3.py F3_SCORE_FILE NXxNY
"""
import json, math, sys
import mpmath as mp

mp.mp.dps = 120
C = mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502',
           '0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')


def jitter(nx, ny, frame, i, j):
    """fraktaler-3 3.1 hybrid.h jitter, subframe 0 (same offset in x and y)."""
    a = (((frame*ny + j)*nx + i)) & 0xffffffff
    M = 0xffffffff
    a = (a + 0x7ed55d16 + (a << 12)) & M
    a = (a ^ 0xc761c23c ^ (a >> 19)) & M
    a = (a + 0x165667b1 + (a << 5)) & M
    a = ((a + 0xd3a2646c) ^ (a << 9)) & M
    a = (a + 0xfd7046c5 + (a << 3)) & M
    a = (a ^ 0xb55a4f09 ^ (a >> 16)) & M
    h = a/4294967296.0; o = 2*h - 1
    v = max(-1.0, o/math.sqrt(abs(o))) if o else -1.0
    return v - (1.0 if o >= 0 else -1.0)


def nu_true(c, px):
    """(nu, de in pixels of size px): de = |z| ln|z| / |dz/dc| / px at escape."""
    z = mp.mpc(0); dz = mp.mpc(0)
    for n in range(1, 20001):
        dz = 2*z*dz + 1; z = z*z + c
        if abs(z) > 1e10:
            return n + 1 - float(mp.log(mp.log(abs(z), 2), 2)), float(abs(z)*mp.log(abs(z))/abs(dz)/px)
    return None, None


def main():
    nx, ny = map(int, sys.argv[2].split('x')); S = 1e-25           # koenigs_bench constants `scale`
    for line in open(sys.argv[1]):
        if not line.startswith('{'):
            continue
        r = json.loads(line); W = float(r['width']); h = W/nx
        for p in r['worst'][:3]:
            i, j = p['i'], ny - 1 - p['row']
            d = jitter(nx, ny, 0, i, j)
            # the bench's exact sample: f64 position, f64 offset in scale units (ill-conditioned
            # near the boundary, so the last-bit rounding must match)
            x = (i + 0.5 + d - nx/2)*h; y = (j + 0.5 + d - ny/2)*h
            c = C + mp.mpc(x/S, y/S)*mp.mpf(S)
            t, de = nu_true(c, mp.mpf(W)/nx); k = math.log(2)*2*de    # 2*de: fd's DE convention
            print(f"w={r['width']} i={i} row={p['row']}: true DE {2*de:.2g} px | dnu f3 {p['nu_f3'] - t:+.2e}"
                  f" ours {p['nu_ours'] - t:+.2e} | px f3 {abs(p['nu_f3'] - t)*k:.1e} ours {abs(p['nu_ours'] - t)*k:.1e}")


if __name__ == '__main__':
    main()
