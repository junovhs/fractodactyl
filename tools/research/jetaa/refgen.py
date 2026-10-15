#!/usr/bin/env python3
"""Reference orbit Z_0 = 0, Z_{n+1} = Z_n^2 + C at the exact decimal centre, for the Q6 jet-AA probe.

usage: refgen.py RE IM WIDTH MAX_ITER OUT

Precision: log2(1/width) + 160 bits. The orbit stops after MAX_ITER steps or once |Z| > 2^16
(samples rebase to Z_0 when they reach the end). OUT layout (little endian): b"JREF0001",
u32 count, then per point f64 re_hi, re_lo, im_hi, im_lo (hi + lo is the point to ~106 bits).
"""
import math
import struct
import sys

import gmpy2
from gmpy2 import mpfr


def main():
    re_s, im_s, width_s, max_iter_s, out = sys.argv[1:6]
    width = float(width_s)
    bits = int(-math.log2(width)) + 160
    gmpy2.get_context().precision = bits
    cr, ci = mpfr(re_s), mpfr(im_s)
    x, y = mpfr(0), mpfr(0)
    pts = [(0.0, 0.0, 0.0, 0.0)]

    def split(v):
        hi = float(v)
        lo = float(v - mpfr(hi))
        return hi, lo

    for _ in range(int(max_iter_s)):
        x, y = x * x - y * y + cr, 2 * x * y + ci
        rh, rl = split(x)
        ih, il = split(y)
        pts.append((rh, rl, ih, il))
        if rh * rh + ih * ih > 2.0 ** 32:
            break
    with open(out, "wb") as f:
        f.write(b"JREF0001")
        f.write(struct.pack("<I", len(pts)))
        for p in pts:
            f.write(struct.pack("<4d", *p))
    print(f"points={len(pts)} bits={bits}")


if __name__ == "__main__":
    main()
