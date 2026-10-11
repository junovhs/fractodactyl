"""AUTO-02 fresh blind pool: minibrots found near the run-1 seed views.

Rule, fixed before any result was seen:
- Seeds: the 14 run-1 views from PROB-16 and FEATURE-MAP (not the AUTO-01 rows).
- At each seed centre, iterate z -> z^2 + c and take the last three atom-domain
  partials (steps n where |z_n| is a new minimum) with period <= MAX_P as candidate
  periods.
- Newton-refine the period-p nucleus from the seed centre at full precision
  (DEC-21: mpmath, never f64), and keep it if it converges.
- Estimate the minibrot size with the usual formula: size = 1/(beta*lambda^2).
- Camera: width = 4 * size, centre = nucleus + (0.2w, -0.15w), so frames hold
  interior and boundary and are off-centre. Iterations: 100000.
Admission is then by BLA only (auto02_precheck.py --mode admit), before the
frozen compiler sees any view.

usage: python3 auto02_minibrots.py CORPUS OUT
"""
import sys

import mpmath as mp

MAX_P = 20000
SEED_ITERS = 100000
SEED_ORIGINS = ("PROB-16", "FEATURE-MAP")


def seeds(corpus):
    for line in open(corpus):
        if line.startswith("#") or not line.strip():
            continue
        name, re, im, width, origin = line.rstrip("\n").split("|")[:5]
        if origin.startswith(SEED_ORIGINS):
            yield name, re, im, width


def partials(c, n_max):
    z, best, out = mp.mpc(0), mp.inf, []
    for n in range(1, n_max + 1):
        z = z * z + c
        a = abs(z)
        if a < best:
            best = a
            out.append(n)
        if a > 2:
            break
    return out


def nucleus(c, p, steps=60):
    eps = mp.mpf(10) ** (-mp.mp.dps + 10)
    for _ in range(steps):
        z, dz = mp.mpc(0), mp.mpc(0)
        for _ in range(p):
            dz = 2 * z * dz + 1
            z = z * z + c
        step = z / dz
        c -= step
        if abs(step) < eps * max(1, abs(c)):
            return c
    return None


def size(c, p):
    z, lam, beta = mp.mpc(0), mp.mpc(1), mp.mpc(1)
    for _ in range(1, p):
        z = z * z + c
        lam = 2 * z * lam
        beta += 1 / lam
    return abs(1 / (beta * lam * lam))


def main(corpus, out):
    rows = []
    for name, re, im, width in seeds(corpus):
        mp.mp.dps = max(40, int(-mp.log10(mp.mpf(width))) + 30)
        c0 = mp.mpc(mp.mpf(re), mp.mpf(im))
        ps = [p for p in partials(c0, SEED_ITERS) if p <= MAX_P][-3:]
        for p in ps:
            c = nucleus(c0, p)
            if c is None:
                print(f"{name} p={p}: newton did not converge", file=sys.stderr)
                continue
            s = size(c, p)
            mp.mp.dps = max(mp.mp.dps, int(-mp.log10(s)) + 30)
            w = 4 * s
            cre = c.real + mp.mpf("0.2") * w
            cim = c.imag - mp.mpf("0.15") * w
            digits = int(-mp.log10(w)) + 20
            row = (f"mb-{name}-p{p}|{mp.nstr(cre, digits, strip_zeros=False)}|"
                   f"{mp.nstr(cim, digits, strip_zeros=False)}|{mp.nstr(w, 6)}|"
                   f"minibrot p{p} near {name}|100000")
            print(f"{name} p={p} size={mp.nstr(s, 3)}", file=sys.stderr)
            rows.append(row)
    with open(out, "w") as f:
        f.write("# AUTO-02 fresh pool: minibrots near run-1 seeds (auto02_minibrots.py rule).\n")
        f.write("# name|re|im|width|origin|fixed_iterations\n")
        f.write("\n".join(rows) + "\n")


if __name__ == "__main__":
    main(*sys.argv[1:3])
