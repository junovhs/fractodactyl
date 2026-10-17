"""PROB-30 fixture: mpmath's v0 nucleus orbit for fd-fixed's Float test.

From v0's c_exact (bench/zones/v0-core.zone), iterate z <- z^2 + C, dz <- 2 z dz + 1 for
the period (764 steps) at DPS digits, and print z_P and z_P / dz_P (the Newton step) to
DPS digits. Also prints how many digits survive a precision change (DPS vs DPS + 40),
which bounds the agreement any other implementation can reach.
Usage: python float_fixture.py [DPS=400] > crates/fd-fixed/tests/fixtures/v0_orbit.txt
"""
import pathlib
import sys
import mpmath as mp

dps = int(sys.argv[1]) if len(sys.argv) > 1 else 400
zone = pathlib.Path(__file__).resolve().parents[3] / "bench/zones/v0-core.zone"
fields = {l.split()[0]: l.split()[1:] for l in zone.read_text().splitlines() if l.strip()}
re, im = fields["c_exact"]
period = int(fields["period"][0])


def run(digits):
    mp.mp.dps = digits
    c = mp.mpc(re, im)
    z = dz = mp.mpc(0)
    for _ in range(period):
        dz = 2 * z * dz + 1
        z = z * z + c
    return z, z / dz


z, step = run(dps)
z2, step2 = run(dps + 40)
mp.mp.dps = dps + 40
agree = lambda a, b: int(-mp.log10(abs(a - b) / abs(b)))
print(f"# float_fixture.py {dps}: v0 c_exact, {period} steps")
print(f"dps {dps}")
print(f"period {period}")
mp.mp.dps = dps
sci = lambda x: mp.nstr(x, dps, min_fixed=0, max_fixed=0)
print(f"c {re} {im}")
print(f"z {sci(z.real)} {sci(z.imag)}")
print(f"step {sci(step.real)} {sci(step.imag)}")
mp.mp.dps = dps + 40
print(f"agree_digits {min(agree(z, z2), agree(step, step2))}")
