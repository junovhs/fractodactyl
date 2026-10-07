"""Oracle for .fds sample files (BASE-02, DEC-09: Python is oracle-only).

Independent of the Rust engine on purpose: reads the file from the format spec
(docs/spec/SAMPLES.md), recomputes a K x K grid of samples by *direct* iteration
z <- z^2 + c in mpmath at (depth + 128) bits, with no perturbation, rebasing or
reference orbit, and compares class, nu, de and normal.

The nu error is gated as an equivalent displacement in output pixels, |dnu| / |dnu/dpx|
with |dnu/dpx| = 2 / (de ln 2): near the boundary nu is steep, so a raw nu tolerance
would only measure how close a sample sits to the set. Raw nu error is still reported.

de and normal keep their direct tolerances (--de relative, --normal radians), but a
sample that misses one fails only if the miss is also more than an --px displacement
(FIX-01, docs/spec/SAMPLES.md "Oracle tolerances"): the error, less its encoding
rounding, is divided by the exact rate the value changes per output pixel, from the
second derivative z'' iterated alongside z'. Near the set a within-contract sub-pixel
shift of the f64 tier moves de and the normal arbitrarily far; that is not a kernel
error. Each such displacement is reported as de_px_err / normal_px_err.

usage: python tools/oracle.py file.fds [--k 8] [--px 1e-3] [--de 1e-2] [--normal 1e-3]
Exit 0 when every sampled value is within tolerance, 1 otherwise.
"""
import argparse
import json
import math
import struct
import sys

from mpmath import mp, mpf

WIDTH = {0: 1, 1: 8, 2: 4, 3: 2, 4: 4}  # bytes per element by column bit
FMT = {0: "<B", 1: "<d", 2: "<f", 3: "<H", 4: "<f"}


def align8(n):
    return (n + 7) & ~7


def read_header(buf):
    magic, major, minor, cols, nx, ny, ss, _, max_iter, radius, rot = struct.unpack_from("<8sHHIIIIIQdd", buf, 0)
    if magic != b"FDSAMPLE" or major != 1:
        raise SystemExit("not a v1 .fds file")
    pos, strs = 56, []
    for _ in range(4):
        (n,) = struct.unpack_from("<H", buf, pos)
        strs.append(buf[pos + 2 : pos + 2 + n].decode())
        pos += 2 + n
    re_, im_, width, kernel = strs
    offsets, off = {}, align8(pos)
    for bit in range(5):
        if cols & (1 << bit):
            offsets[bit] = off
            off += align8(WIDTH[bit] * nx * ny)
    return dict(nx=nx, ny=ny, ss=ss, max_iter=max_iter, radius=radius, rot=rot,
                re=re_, im=im_, width=width, kernel=kernel, offsets=offsets)


def column(buf, h, bit, idx):
    if bit not in h["offsets"]:
        return None
    return struct.unpack_from(FMT[bit], buf, h["offsets"][bit] + WIDTH[bit] * idx)[0]


def in_main_components(cr, ci):
    y2 = ci * ci
    q = (cr - mpf(1) / 4) ** 2 + y2
    return q * (q + (cr - mpf(1) / 4)) <= y2 / 4 or (cr + 1) ** 2 + y2 <= mpf(1) / 16


def direct(cr, ci, max_iter, r2):
    """Direct iteration with first and second c-derivatives.
    Returns (n, zr, zi, dr, di, sr, si) or None."""
    zr = zi = dr = di = sr = si = mpf(0)
    for n in range(1, max_iter + 1):
        sr, si = 2 * (dr * dr - di * di + zr * sr - zi * si), 2 * (2 * dr * di + zr * si + zi * sr)
        dr, di = 2 * (zr * dr - zi * di) + 1, 2 * (zr * di + zi * dr)
        zr, zi = zr * zr - zi * zi + cr, 2 * zr * zi + ci
        if zr * zr + zi * zi > r2:
            return n, zr, zi, dr, di, sr, si
    return None


def check(path, k, tol):
    buf = open(path, "rb").read()
    h = read_header(buf)
    nx, ny = h["nx"], h["ny"]
    mp.prec = 64
    hsp = mpf(h["width"]) / nx  # sample spacing
    depth = max(0, -int(math.floor(float(mp.log(hsp, 2)))))
    mp.prec = depth + 128
    hsp = mpf(h["width"]) / nx
    px = hsp * h["ss"]
    c0r, c0i = mpf(h["re"]), mpf(h["im"])
    cos, sin = mp.cos(mpf(h["rot"])), mp.sin(mpf(h["rot"]))
    r2 = mpf(h["radius"]) ** 2
    stats = dict(file=path, kernel=h["kernel"], samples=0, escaped=0, class_mismatch=0,
                 nu_err=0.0, nu_px_err=0.0, de_rel_err=0.0, de_px_err=0.0, normal_err=0.0, normal_px_err=0.0, bits=mp.prec, failures=[])
    for a in range(k):
        for b in range(k):
            i = (2 * a + 1) * nx // (2 * k)
            j = (2 * b + 1) * ny // (2 * k)
            idx = j * nx + i
            x, y = mpf(i) + mpf(1) / 2 - mpf(nx) / 2, mpf(j) + mpf(1) / 2 - mpf(ny) / 2
            cr = c0r + hsp * (cos * x + sin * y)
            ci = c0i + hsp * (sin * x - cos * y)
            kind = column(buf, h, 0, idx) & 3  # 0 escaped, 1 interior, 2 unresolved
            stats["samples"] += 1
            res = None if in_main_components(cr, ci) else direct(cr, ci, h["max_iter"], r2)
            if (res is not None) != (kind == 0):
                stats["class_mismatch"] += 1
                stats["failures"].append(dict(i=i, j=j, rust_kind=kind, oracle_escaped=res is not None))
                continue
            if res is None:
                continue
            stats["escaped"] += 1
            n, zr, zi, dr, di, sr, si = res
            az = mp.sqrt(zr * zr + zi * zi)
            nu = n + 1 - mp.log(mp.log(az, 2), 2)
            de = float(2 * az * mp.log(az) / mp.sqrt(dr * dr + di * di) / px)
            got = column(buf, h, 1, idx)
            if got is not None:
                err = abs(float(nu) - got)
                stats["nu_err"] = max(stats["nu_err"], err)
                stats["nu_px_err"] = max(stats["nu_px_err"], err * de * math.log(2) / 2)
            # Rates per output pixel from the c-gradients (|grad Re f| = |f'| for
            # holomorphic f): ln|z| -> z'/z, ln|z'| -> z''/z'.
            m2, d2 = zr * zr + zi * zi, dr * dr + di * di
            gr, gi = (zr * dr + zi * di) / m2, (zr * di - zi * dr) / m2  # z'/z
            hr, hi = (sr * dr + si * di) / d2, (si * dr - sr * di) / d2  # z''/z'
            got = column(buf, h, 2, idx)
            if got is not None:
                if math.isfinite(got) and de > 0:
                    rel = abs(got - de) / de
                    stats["de_rel_err"] = max(stats["de_rel_err"], rel)
                    # ln de = ln 2 + ln|z| + ln ln|z| - ln|z'|: |grad ln de| = |(z'/z)(1 + 1/ln|z|) - z''/z'|.
                    f = 1 + 1 / mp.log(az)
                    grad = de * float(mp.sqrt((gr * f - hr) ** 2 + (gi * f - hi) ** 2) * px)
                    excess = abs(got - de) - de * 2.0 ** -24  # f32 rounding
                    if rel > tol["de"] and excess > 0:
                        stats["de_px_err"] = max(stats["de_px_err"], excess / grad if grad > 0 else math.inf)
            got = column(buf, h, 3, idx)
            if got is not None:
                wr, wi = zr * dr + zi * di, zi * dr - zr * di
                ang = float(mp.atan2(sin * wr - cos * wi, cos * wr + sin * wi))
                diff = abs((got * 2 * math.pi / 65536 - ang + math.pi) % (2 * math.pi) - math.pi)
                stats["normal_err"] = max(stats["normal_err"], diff)
                # |d arg(w)/dc| = |w'/w| for holomorphic w = z/z'; w'/w = z'/z - z''/z'.
                rate = float(mp.sqrt((gr - hr) ** 2 + (gi - hi) ** 2) * px)  # rad per output pixel
                excess = diff - math.pi / 65536  # u16 rounding
                if diff > tol["normal"] and excess > 0:
                    stats["normal_px_err"] = max(stats["normal_px_err"], excess / rate if rate > 0 else math.inf)
    ok = (stats["class_mismatch"] == 0 and stats["nu_px_err"] <= tol["px"]
          and stats["de_px_err"] <= tol["px"] and stats["normal_px_err"] <= tol["px"])
    stats["ok"] = ok
    return stats


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("file")
    ap.add_argument("--k", type=int, default=8)
    ap.add_argument("--px", type=float, default=1e-3)
    ap.add_argument("--de", type=float, default=1e-2)
    ap.add_argument("--normal", type=float, default=1e-3)
    a = ap.parse_args()
    stats = check(a.file, a.k, dict(px=a.px, de=a.de, normal=a.normal))
    print(json.dumps(stats))
    sys.exit(0 if stats["ok"] else 1)


if __name__ == "__main__":
    main()
