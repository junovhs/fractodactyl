"""PROB-14: high-precision v0 mid-band constants for koenigs-bench --mid.

Usage: python tools/research/misiurewicz/mid_consts.py OUT
All orbit, coefficient and multiplier powers are rounded only on file output.
The derivative coefficients follow equation (4) in
docs/research/10-9-26/parameter-taylor-truncation-of-the-koenigs-coefficients.md.
"""
import sys
import mpmath as mp

mp.mp.dps = 100
RE = "-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502"
IM = "0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922"
C = mp.mpc(RE, IM)
N = 18
P = 764
J = 512


def mul(a, da, b, db, n):
    z = [mp.mpc(0) for _ in range(n + 1)]
    dz = z.copy()
    for i in range(n + 1):
        for j in range(n + 1 - i):
            z[i + j] += a[i] * b[j]
            dz[i + j] += da[i] * b[j] + a[i] * db[j]
    return z, dz


def coeffs(p, lam):
    q = -1 - p
    pprime = -1 / (2 * p + 1)
    a, ap = 4 * p * p + 2 * q, (8 * p - 2) * pprime
    b, bp = 4 * p, 4 * pprime
    k = [mp.mpc(0) for _ in range(N + 1)]
    kp = k.copy()
    k[1] = 1
    for n in range(2, N + 1):
        k2, k2p = mul(k, kp, k, kp, n)
        k3, k3p = mul(k2, k2p, k, kp, n)
        k4, k4p = mul(k2, k2p, k2, k2p, n)
        den = lam**n - lam
        denp = 4 * (n * lam**(n - 1) - 1)
        qn = a * k2[n] + b * k3[n] + k4[n]
        k[n] = qn / den
        qnp = ap * k2[n] + a * k2p[n] + bp * k3[n] + b * k3p[n] + k4p[n]
        kp[n] = (qnp - denp * k[n]) / den
    return k, kp


def main(out):
    z = mp.mpc(0)
    orbit = [z]
    for _ in range(P):
        z = z*z + C
        orbit.append(z)
    s = mp.sqrt(-3 - 4*C)
    pplus, pminus = (-1+s)/2, (-1-s)/2
    sign = 1 if abs(orbit[24] - pplus) < abs(orbit[24] - pminus) else -1
    p = pplus if sign == 1 else pminus
    lam = 4 * (1 + C)
    k, kp = coeffs(p, lam)
    enc = lambda z: f"{float(mp.re(z))!r} {float(mp.im(z))!r}"
    rows = [
        "# PROB-14: mid-band zone, 100-digit constants stored as f64",
        f"c_exact {RE} {IM}",
        f"c {enc(C)}",
        f"u {enc(orbit[24] - p)}",
        f"s {enc(s)}",
        f"p {enc(p)}",
        f"sign {sign}",
        f"lambda {enc(lam)}",
    ]
    rows += [f"z {i} {enc(z)}" for i, z in enumerate(orbit[:25])]
    rows += [f"ref {i} {enc(z)}" for i, z in enumerate(orbit[:P])]
    rows += [f"k {i} {enc(z)}" for i, z in enumerate(k)]
    rows += [f"kp {i} {enc(z)}" for i, z in enumerate(kp)]
    power = mp.mpc(1)
    for j in range(J):
        rows.append(f"t {j} {enc(power)}")
        power *= lam
    with open(out, "w", encoding="utf-8") as file:
        file.write("\n".join(rows) + "\n")
    print(f"wrote {out}: approach 24, return {P}, k and dk {N}, T_j {J}")


if __name__ == "__main__":
    main(sys.argv[1])
