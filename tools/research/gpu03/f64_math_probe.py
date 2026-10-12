#!/usr/bin/env python3
"""Scalar binary64 check of zone_f64.comp's math (not a Vulkan parity test)."""
import math
import random

LN2 = 0.693147180559945309417232121458
INV_LN2 = 1.442695040888963407359924681002
LN2_LO = 2.3190468138462995584e-17
PI_2 = 1.570796326794896619231321691640
PI_2_LO = 6.1232339957367660359e-17
INV_PI_2 = 0.636619772367581343075535053490


def gpu_log(x):
    m, e = math.frexp(x)
    m *= 2
    e -= 1
    if m > math.sqrt(2):
        m *= 0.5
        e += 1
    t = (m-1)/(m+1)
    term = total = t
    for k in range(1, 12):
        term *= t*t
        total += term/(2*k+1)
    return e*LN2 + 2*total


def gpu_exp(x):
    k = round(x*INV_LN2)
    r = math.fma(-k, LN2, x)
    r = math.fma(-k, LN2_LO, r)
    p = 1.0
    for j in range(20, 0, -1):
        p = 1 + r*p/j
    return math.ldexp(p, k)


def atan_small(z):
    p = -1/47
    for k in range(22, -1, -1):
        p = (1 if k % 2 == 0 else -1)/(2*k+1) + z*z*p
    return z*p


def atan_unit(x):
    return (math.pi/4 + atan_small((x-1)/(x+1))
            if x > math.sqrt(2)-1 else atan_small(x))


def gpu_atan2(y, x):
    ax, ay = abs(x), abs(y)
    if ax == ay == 0:
        return 0.0
    a = math.pi/2-atan_unit(ax/ay) if ay > ax else atan_unit(ay/ax)
    if x < 0:
        a = math.pi-a
    return -a if y < 0 else a


def gpu_sincos(x):
    q = round(x*INV_PI_2)
    r = math.fma(-q, PI_2, x)
    r = math.fma(-q, PI_2_LO, r)
    rr = r*r
    sc = -1/10888869450418352160768000000
    cc = -1/403291461126605635584000000
    sp, cp = sc, cc
    for k in range(12, -1, -1):
        sc *= -((2*k+2)*(2*k+3))
        cc *= -((2*k+1)*(2*k+2))
        sp, cp = sp*rr+sc, cp*rr+cc
    s, c = r*sp, cp
    return ((s, c), (c, -s), (-s, -c), (-c, s))[q % 4]


def main():
    rng = random.Random(53)
    maxima = dict.fromkeys(("log", "log2", "exp_rel", "sin", "cos", "atan2"), 0.0)

    def record(name, error):
        maxima[name] = max(maxima[name], abs(error))

    # Keep the order/ranges and seed fixed for repeatability.
    for _ in range(90_000):
        x = (rng.random()+0.25)*10**rng.uniform(-295, 295)
        a = gpu_log(x)
        record("log", a-math.log(x))
        record("log2", a*INV_LN2-math.log2(x))
        x = rng.uniform(-680, 680)
        record("exp_rel", gpu_exp(x)/math.exp(x)-1)
        x = rng.uniform(-1500, 1500)
        s, c = gpu_sincos(x)
        record("sin", s-math.sin(x))
        record("cos", c-math.cos(x))
        y, x = rng.uniform(-1, 1), rng.uniform(-1, 1)
        record("atan2", gpu_atan2(y, x)-math.atan2(y, x))

    limits = dict(log=2e-13, log2=4e-13, exp_rel=5e-16,
                  sin=3e-16, cos=3e-16, atan2=1e-15)
    for name, error in maxima.items():
        print(f"{name}: max {error:.9g}; limit {limits[name]:.1g}")
        assert error <= limits[name], f"{name}: outside scalar binary64 error budget"


if __name__ == "__main__":
    main()
