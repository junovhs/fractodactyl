"""Write the per-zone constants for the Rust koenigs_bench (PROB-08).

All are computed once per zone at high precision and rounded to double:
- the PROB-07 biseries coefficients a_ij (u^i v^j, scale 1e-25) and the nucleus residual;
- C's critical orbit Z_1..Z_24 and Z_24 - alpha;
- the 2-cycle point alpha, its multiplier rho and the Koenigs series phi;
- C's periodic reference orbit z_0..z_{P-1}, for the lean perturbation baseline.
Usage: python koenigs_bench_consts.py DEGREE GUARD TERMS R0 OUT [PSI_TERMS=0]
(PSI_TERMS > 0 also writes psi = phi^-1 by series reversion, PROB-09; 0 means Newton.)
"""
import sys, time
import mpmath as mp
deg, guard, terms, r0, out = int(sys.argv[1]), sys.argv[2], int(sys.argv[3]), sys.argv[4], sys.argv[5]
psi_terms = int(sys.argv[6]) if len(sys.argv) > 6 else 0
sys.argv = [sys.argv[0], 'unused']
import koenigs_tail as kt, returns_exit_tail as rt, psi_series

t = time.perf_counter()
coeff, bias, _ = rt.build_map(deg)
mp.mp.dps = kt.DPS
A, RHO, COEF = kt.setup()
Z = [kt.C]
for _ in range(23): Z.append(Z[-1]**2 + kt.C)
c = lambda z: f"{float(mp.re(z))!r} {float(mp.im(z))!r}"
lines = [f"# koenigs_bench constants: degree {deg} guard {guard} terms {terms} r0 {r0}",
         f"c {c(kt.C)}", f"period {kt.P}", f"scale {float(rt.SCALE)!r}", f"guard {float(guard)!r}",
         f"r0 {float(r0)!r}", f"bias {c(rt.decode(bias))}", f"alpha {c(A)}", f"rho {c(RHO)}",
         f"z24_minus_alpha {c(Z[23] - A)}"]
lines += [f"orbit {i} {c(z)}" for i, z in enumerate(Z[:23])]
# Lean perturbation baseline: C's periodic reference orbit z_0 = 0 .. z_{P-1}.
R = [mp.mpc(0)]
for _ in range(kt.P - 1): R.append(R[-1]**2 + kt.C)
lines += [f"ref {i} {c(z)}" for i, z in enumerate(R)]
lines += [f"biseries {i} {j} {c(rt.decode(a))}" for i, j, a in coeff]
lines += [f"phi {k} {c(COEF[k])}" for k in range(1, terms + 1)]
if psi_terms:
    PSI = psi_series.reverse(COEF, psi_terms)
    lines += [f"psi {k} {c(PSI[k])}" for k in range(1, psi_terms + 1)]
open(out, 'w').write('\n'.join(lines) + '\n')
print(f"wrote {out}: {len(coeff)} biseries terms, {terms} phi terms, {time.perf_counter() - t:.2f} s")
