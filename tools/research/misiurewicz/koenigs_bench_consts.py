"""Write the per-zone constants for the Rust koenigs_bench (PROB-08).

All are computed once per zone at high precision and rounded to double:
- the PROB-07 biseries coefficients a_ij (u^i v^j, scale 1e-25) and the nucleus residual;
- C's critical orbit Z_1..Z_24 and Z_24 - alpha;
- the biseries and Z_24 - alpha again as mantissa + binary exponent (`_x` lines, PROB-19),
  exact at ladder rungs where the doubles overflow or underflow;
- the 2-cycle point alpha, its multiplier rho and the Koenigs series phi;
- C's periodic reference orbit z_0..z_{P-1}, for the lean perturbation baseline.
Usage: python koenigs_bench_consts.py DEGREE GUARD TERMS R0 OUT [PSI_TERMS=0] [RE IM PERIOD]
(PSI_TERMS > 0 also writes psi = phi^-1 by series reversion, PROB-09; 0 means Newton.)
"""
import sys, time
import mpmath as mp
deg, guard, terms, r0, out = int(sys.argv[1]), sys.argv[2], int(sys.argv[3]), sys.argv[4], sys.argv[5]
psi_terms = int(sys.argv[6]) if len(sys.argv) > 6 else 0
centre = sys.argv[7:]
if len(centre) not in (0, 3):
    raise SystemExit("expected RE IM PERIOD together")
sys.argv = [sys.argv[0], 'unused']
import koenigs_tail as kt, returns_exit_tail as rt, psi_series

t = time.perf_counter()
if centre:
    re, im, period = centre
    period = int(period)
    if period < 26:
        raise SystemExit("period must be at least 26")
    # Preserve enough digits to form the tiny nucleus residual and size.
    # The period-16116 reference loses roughly 1000 digits over an orbit.
    # Refine the supplied nucleus before forming the reference or its map.
    depth = 50 + max(0, period - 764) // 16
    dps = max(kt.DPS, 2 * depth + 70)
    mp.mp.dps = dps
    kt.DPS = rt.DPS = dps
    c0 = mp.mpc(re, im)
    for _ in range(8):
        z = dz = mp.mpc(0)
        for _ in range(period):
            dz = 2*z*dz + 1
            z = z*z + c0
        step = z / dz
        c0 -= step
        if abs(step) < mp.power(10, -(dps - 25)):
            break
    else:
        raise ValueError("nucleus Newton refinement did not converge")
    kt.C = rt.C = c0
    kt.P = rt.P = period
    # Return-map state offsets scale as the square root of minibrot size.
    # Parameter offsets are divided by the same scale in the Rust zone.
    z = dz = mp.mpc(0)
    b = mp.mpc(1)
    for j in range(period):
        dz = 2*z*dz + 1
        z = z*z + kt.C
        if j < period - 1:
            b *= 2*z
    size = abs(1 / (b * dz))
    rt.SCALE = mp.power(10, int(mp.floor(mp.log10(size) / 2)))
    if guard == "auto":
        guard = mp.nstr(rt.SCALE * mp.mpf('1e-3'), 16)
elif guard == "auto":
    guard = "1e-28"
coeff, bias, _ = rt.build_map(deg)
mp.mp.dps = kt.DPS
A, RHO, COEF = kt.setup()
Z = [kt.C]
for _ in range(23): Z.append(Z[-1]**2 + kt.C)
c = lambda z: f"{float(mp.re(z))!r} {float(mp.im(z))!r}"


def fx(z):
    """z as mantissa pair and shared binary exponent: deep rungs leave f64 (DEC-21)."""
    a = max(abs(mp.re(z)), abs(mp.im(z)))
    if a == 0:
        return "0.0 0.0 0"
    e = int(mp.frexp(a)[1]) - 1
    return f"{float(mp.ldexp(mp.re(z), -e))!r} {float(mp.ldexp(mp.im(z), -e))!r} {e}"


lines = [f"# koenigs_bench constants: degree {deg} guard {guard} terms {terms} r0 {r0}",
         f"c_exact {mp.nstr(mp.re(kt.C), kt.DPS - 2)} {mp.nstr(mp.im(kt.C), kt.DPS - 2)}",
         f"c {c(kt.C)}", f"period {kt.P}", f"scale {mp.nstr(rt.SCALE, 18)}", f"guard {guard}",
         f"r0 {float(r0)!r}", f"bias {c(rt.decode(bias))}", f"alpha {c(A)}", f"rho {c(RHO)}",
         f"z24_minus_alpha {c(Z[23] - A)}", f"z24_minus_alpha_x {fx(Z[23] - A)}"]
lines += [f"orbit {i} {c(z)}" for i, z in enumerate(Z[:23])]
# Lean perturbation baseline: C's periodic reference orbit z_0 = 0 .. z_{P-1}.
R = [mp.mpc(0)]
for _ in range(kt.P - 1): R.append(R[-1]**2 + kt.C)
lines += [f"ref {i} {c(z)}" for i, z in enumerate(R)]
lines += [f"biseries {i} {j} {c(rt.decode(a))}" for i, j, a in coeff]
lines += [f"biseries_x {i} {j} {fx(rt.decode(a))}" for i, j, a in coeff]
lines += [f"phi {k} {c(COEF[k])}" for k in range(1, terms + 1)]
if psi_terms:
    PSI = psi_series.reverse(COEF, psi_terms)
    lines += [f"psi {k} {c(PSI[k])}" for k in range(1, psi_terms + 1)]
open(out, 'w').write('\n'.join(lines) + '\n')
print(f"wrote {out}: {len(coeff)} biseries terms, {terms} phi terms, {time.perf_counter() - t:.2f} s")
