import math, sys
re_, im_ = sys.argv[1], sys.argv[2]
fps, secs = 24, 10
w0, decades, turns = 4.0, 5.0, 1.25
n = fps * secs
print(f"# 10 s twist zoom, {fps} fps, eased speed, {turns} turns")
for f in range(n):
    s = f / (n - 1)
    e = s * s * (3 - 2 * s)            # smoothstep: slow start, faster middle, slow end
    w = w0 * 10 ** (-decades * e)
    rot = 2 * math.pi * turns * e
    print(f"{re_} {im_} {float(f'{w:.6g}')!r} {rot!r}")
