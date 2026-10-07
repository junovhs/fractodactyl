"""PROB-12 seed test: along a fixed-centre zoom path, which depth bands have the v0-style
structure (a minibrot whose orbit circles one repelling cycle for many laps)?

For each width w (one per decade): p = the lowest period whose Newton distance
|z_p/u_p| at the centre is below w (the lowest-period minibrot in view), Newton to its
nucleus, then the longest run where the nucleus orbit repeats after r steps near a
repelling cycle (zone_seed.dwell_any): laps = run / r, skippable fraction = run / p.
Usage: python path_laps.py NAME RE IM MIN_WIDTH_EXP [DPS=150] [PMAX=5000]
"""
import sys, time
import mpmath as mp
name, RE, IM, wmin = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
mp.mp.dps = int(sys.argv[5]) if len(sys.argv) > 5 else 150
PMAX = int(sys.argv[6]) if len(sys.argv) > 6 else 5000
sys.argv = [sys.argv[0]]
import zone_seed as zs
zs.mp.mp.dps = mp.mp.dps


def main():
    t0 = time.time(); c = mp.mpc(RE, IM)
    z, u = [mp.mpc(0)], [mp.mpc(0)]
    for _ in range(PMAX):
        u.append(2*z[-1]*u[-1] + 1); z.append(z[-1]**2 + c)
        if abs(z[-1]) > 1e6: break
    dist = [abs(z[n]/u[n]) if u[n] != 0 else mp.inf for n in range(len(z))]
    print(f"{name}: orbit {len(z) - 1} steps")
    print(f"{'width':>7} {'p':>5} {'|c-cH|':>9} {'r':>4} {'laps':>6} {'skip/p':>6}")
    last = None
    for e in range(3, -wmin + 1, 3):
        w = mp.mpf(10)**(-e)
        p = next((n for n in range(1, len(z)) if dist[n] < w), None)
        if p is None: print(f"  1e-{e:<3} (no minibrot within {PMAX} steps)"); continue
        if p == last: print(f"  1e-{e:<3} {p:5d}   (same)"); continue
        last = p; cH = c
        for _ in range(80):
            zz, uu = mp.mpc(0), mp.mpc(0)
            for _ in range(p): uu = 2*zz*uu + 1; zz = zz*zz + cH
            st = zz/uu; cH -= st
            if abs(st) < mp.mpf(10)**(-mp.mp.dps + 10): break
        run, start, r, lam = zs.dwell_any(cH, p) if p > 3 else (0, 0, 0, 0)
        laps = run/r if r else 0
        print(f"  1e-{e:<3} {p:5d} {mp.nstr(abs(c - cH), 2):>9} {r:4d} {laps:6.1f} {run/p:6.0%}")
    print(f"({time.time() - t0:.1f} s)")


if __name__ == '__main__':
    main()
