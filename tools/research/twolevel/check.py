#!/usr/bin/env python3
"""Adjudicate pixels by direct high-precision iteration. usage: check.py LOC LOG10W x,y [x,y ...]"""
import sys, math
import gmpy2 as g
from gmpy2 import mpfr, mpc
import ref

def centre(loc, lw):
    # mirrors ref.main without writing files
    import io, contextlib, tempfile, os
    d = tempfile.mkdtemp()
    sys.argv = ['ref.py', loc, str(lw), d + '/x.ref', '100000']
    # recompute C exactly as ref.py does
    W = 10.0 ** lw
    K = math.floor(math.log2(W / 1920))
    return K

def main():
    loc, lw = sys.argv[1], float(sys.argv[2])
    pts = [tuple(int(v) for v in a.split(',')) for a in sys.argv[3:]]
    K = math.floor(math.log2(10.0 ** lw / 1920))
    g.get_context().precision = int(-lw * 3.33) + 400
    C = ref.centre_of(loc, lw, 100000)
    h = mpfr(2) ** K
    for (x, y) in pts:
        c = C + mpc((x - 960 + mpfr(0.5)) * h, (y - 540 + mpfr(0.5)) * h)
        z = mpc(0); dz = mpc(0)
        for n in range(1, 100001):
            dz = 2 * z * dz + 1; z = z * z + c
            if abs(z) > 1e4:
                lz = math.log(abs(complex(z)))
                nu = n + 1 - math.log2(lz)
                de = float(abs(z) * lz / abs(dz) / h)
                print(f"px {x},{y} n {n} nu {nu:.6f} de_px {de:.3e}")
                break
        else:
            print(f"px {x},{y} no escape")

if __name__ == '__main__':
    main()
