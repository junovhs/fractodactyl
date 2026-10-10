"""PROB-18: mpmath oracle for pixels that fail fd compare.

Usage: python diagnose_pixels.py FD_FDS ZONE_FDS [--limit 200]
Only samples beyond the FIX-04 Unresolved -> Interior gap are probed.
"""
import argparse
import math
from pathlib import Path
import struct

import mpmath as mp


def load(path):
    b = Path(path).read_bytes()
    if b[:8] != b"FDSAMPLE":
        raise ValueError(f"{path}: not an FDS file")
    _, major, _, mask, nx, ny, ss, _, maxit, radius, rot = struct.unpack_from(
        "<8sHHIIIIIQdd", b)
    if major != 1 or ss != 1 or mask & 15 != 15:
        raise ValueError(f"{path}: expected FDS v1, ss=1, class/nu/de/normal")
    pos = 56
    fields = []
    for _ in range(4):
        n = struct.unpack_from("<H", b, pos)[0]
        pos += 2
        fields.append(b[pos:pos + n].decode())
        pos += n
    pos = (pos + 7) & ~7
    offsets = {}
    for bit, (name, width) in enumerate(
            (("class", 1), ("nu", 8), ("de", 4), ("normal", 2), ("bound", 4))):
        if mask & (1 << bit):
            offsets[name] = pos
            pos += (nx * ny * width + 7) & ~7
    if pos > len(b):
        raise ValueError(f"{path}: truncated columns")
    return dict(data=b, offsets=offsets, nx=nx, ny=ny, maxit=maxit,
                radius=radius, rot=rot, re=fields[0], im=fields[1],
                width=fields[2])


def col(f, name, k):
    offset = f["offsets"][name]
    if name == "class":
        return f["data"][offset + k] & 3
    code, width = {"nu": ("d", 8), "de": ("f", 4),
                   "normal": ("H", 2)}[name]
    return struct.unpack_from("<" + code, f["data"], offset + k * width)[0]


def angle_diff(a, b):
    d = (a - b) % 65536
    return min(d, 65536 - d) * (360 / 65536)


def disputed(a, b, k):
    ca, cb = col(a, "class", k), col(b, "class", k)
    if ca != cb:
        return [] if (ca, cb) == (2, 1) else ["class"]
    if ca != 0:
        return []
    da, db = col(a, "de", k), col(b, "de", k)
    na, nb = col(a, "nu", k), col(b, "nu", k)
    if not all(math.isfinite(v) for v in (da, db, na, nb)) or da < 0 or db < 0:
        return ["nonfinite"]
    issues = []
    if abs(na - nb) * da * math.log(2) / 2 > 1e-3:
        issues.append("nu")
    if da > 1e-3:
        if abs(db - da) / da > 2e-3:
            issues.append("de")
        if angle_diff(col(a, "normal", k), col(b, "normal", k)) > 0.2:
            issues.append("normal")
    return issues


def oracle(f, x, y):
    nx, ny = f["nx"], f["ny"]
    dx, dy = mp.mpf(x) + mp.mpf("0.5") - nx / 2, mp.mpf(y) + mp.mpf("0.5") - ny / 2
    h = mp.mpf(f["width"]) / nx
    rot = mp.mpf(f["rot"])
    co, si = mp.cos(rot), mp.sin(rot)
    c = mp.mpc(mp.mpf(f["re"]) + h * (co * dx + si * dy),
               mp.mpf(f["im"]) + h * (si * dx - co * dy))
    z = dz = mp.mpc(0)
    for n in range(1, f["maxit"] + 1):
        dz = 2 * z * dz + 1
        z = z * z + c
        az = abs(z)
        if az > f["radius"]:
            nu = n + 1 - mp.log(mp.log(az, 2), 2)
            de = 2 * az * mp.log(az) / abs(dz) / h
            normal = int(mp.nint((rot - mp.arg(z / dz)) * 65536 / (2 * mp.pi))) % 65536
            return ("escaped", float(nu), float(de), normal, n)
    return ("unresolved", None, None, None, f["maxit"])


def adjudicate(a, b, limit):
    """Disputed samples beyond FIX-04 (at most `limit`, each against mpmath) and the
    total count. Each row says whose fault the dispute is: the zone's if its class
    differs from mpmath's or, where both escape, its values miss mpmath's."""
    if any(a[k] != b[k] for k in ("nx", "ny", "maxit", "radius", "rot",
                                  "re", "im", "width")):
        raise ValueError("not the same camera and grid")
    rows, total = [], 0
    for k in range(a["nx"] * a["ny"]):
        issues = disputed(a, b, k)
        if not issues:
            continue
        total += 1
        if total > limit:
            continue
        x, y = k % a["nx"], k // a["nx"]
        truth = oracle(a, x, y)
        row = dict(x=x, y=y, issues=issues, fd_class=col(a, "class", k),
                   zone_class=col(b, "class", k), truth=truth[0], n=truth[4], err={})
        if truth[0] == "escaped":
            for name, f in (("fd", a), ("zone", b)):
                if col(f, "class", k) == 0:
                    de = col(f, "de", k)
                    row["err"][name] = dict(
                        nu_px=abs(col(f, "nu", k) - truth[1]) * truth[2] * math.log(2) / 2,
                        de_rel=abs(de - truth[2]) / truth[2],
                        normal_deg=angle_diff(col(f, "normal", k), truth[3]))
        z = row["err"].get("zone")
        if z is not None:
            # Score zone shading against mpmath even when fd did not escape.
            bad = z["nu_px"] > 1e-3 or (truth[2] > 1e-3 and (
                z["de_rel"] > 2e-3 or z["normal_deg"] > 0.2))
        else:
            bad = (row["zone_class"] == 0) != (truth[0] == "escaped")
        row["fault"] = "zone" if bad else "fd"
        rows.append(row)
    return rows, total


def diagnose(a, b, limit):
    rows, total = adjudicate(a, b, limit)
    for r in rows:
        print(f"({r['x']},{r['y']}) {','.join(r['issues'])}: fd class={r['fd_class']} "
              f"zone class={r['zone_class']} mpmath={r['truth']} at n={r['n']} "
              f"-> {r['fault']} fault")
        for name, e in r["err"].items():
            print(f"  {name}: nu-px={e['nu_px']:.3g} de-rel={e['de_rel']:.3g} "
                  f"normal-deg={e['normal_deg']:.3g}")
    print(f"Disputed samples beyond FIX-04: {total}; checked with mpmath: {min(total, limit)}")
    if total > limit:
        print("Not all disputed pixels checked; raise --limit before adjudication.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fd")
    parser.add_argument("zone")
    parser.add_argument("--limit", type=int, default=200)
    parser.add_argument("--dps", type=int, default=180)
    args = parser.parse_args()
    if args.limit < 1 or args.dps < 100:
        parser.error("--limit must be positive and --dps >= 100")
    mp.mp.dps = args.dps
    diagnose(load(args.fd), load(args.zone), args.limit)
