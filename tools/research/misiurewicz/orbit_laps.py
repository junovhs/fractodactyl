"""PROB-12 seed test on a real famous zoom: how much of the centre's orbit is spent circling
repelling cycles (the steps a Koenigs jump skips for every nearby pixel)?

Computes the centre's critical orbit at high precision for N steps, then (in double, numpy)
finds runs where the orbit repeats itself after r steps (|z_{n+r} - z_n| < TOL) for r <= RMAX,
keeps runs of at least MINLAPS laps whose cycle is repelling (|lambda| = prod |2z| > 1), and
reports the union of covered steps plus the biggest structures.
Default location: Maths Town "Eye of the Universe" (zoom 3.4e1091, ~17M iterations).
Usage: python orbit_laps.py [N=200000] [RMAX=3000] [MINLAPS=4] [TOL=1e-3] [LOCFILE or -] [RCAP]
"""
import sys, time
import numpy as np
import mpmath as mp
N = int(sys.argv[1]) if len(sys.argv) > 1 else 200000
RMAX = int(sys.argv[2]) if len(sys.argv) > 2 else 3000
MINLAPS = float(sys.argv[3]) if len(sys.argv) > 3 else 4
TOL = float(sys.argv[4]) if len(sys.argv) > 4 else 1e-3
RCAP = int(sys.argv[6]) if len(sys.argv) > 6 else 10**9      # ignore cycle lengths >= RCAP (e.g. the centre's own minibrot)
RE = ('0.360240443437614363236125244449545308482607807958585750488375814740195346059218100311752936722773426396233731729724987737320035372683285317664532401218521579554288661726564324134702299962817029213329980895208036363104546639698106204384566555001322985619004717862781192694046362748742863016467354574422779443226982622356594130430232458472420816652623492974891730419252651127672782407292315574480207005828774566475024380960675386215814315654794021855269375824443853463117354448779647099224311848192893972572398662626725254769950976527431277402440752868498588785436705371093442460696090720654908973712759963732914849861213100695402602927267843779747314419332179148608587129105289166676461292845685734536033692577618496925170576714796693411776794742904333484665301628662532967079174729170714156810530598764525260869731233845987202037712637770582084286587072766838497865108477149114659838883818795374195150936369987302574377608649625020864292915913378927790344097552591919409137354459097560040374880346637533711271919419723135538377394364882968994646845930838049998854075817859391340445151448381853615103761584177161812057928')
IM = ('-0.6413130610648031748603750151793020665794949522823052595561775430644485741727536902556370230689681162370740565537072149790106973211105273740851993394803287437606238596262287731075999483940467161288840614581091294325709988992269165007394305732683208318834672366947550710920088501655704252385244481168836426277052232593412981472237968353661477793530336607247738951625817755401065045362273039788332245567345061665756708689359294516668271440525273653083717877701237756144214394870245598590883973716531691124286669552803640414068523325276808909040317617092683826521501539932397262012011082098721944643118695001226048977430038509470101715555439047884752058334804891389685530946112621573416582482926221804767466258346014417934356149837352092608891639072745930639364693513216719114523328990690069588676087923656657656023794484324797546024248328156586471662631008741349069961493817600100133439721557969263221185095951241491408756751582471307537382827924073746760884081704887902040036056611401378785952452105099242499241003208013460878442953408648178692353788153787229940221611731034405203519945313911627314900851851072122990492499999999999999999991')


def main():
    if len(sys.argv) > 5 and sys.argv[5] != '-':
        re, im = open(sys.argv[5]).read().split()[:2]
    else:
        re, im = RE, IM
    mp.mp.dps = max(len(re), len(im)) + 50
    t0 = time.time(); c = mp.mpc(re, im); z = mp.mpc(0)
    zd = np.empty(N + 1, complex); zd[0] = 0; esc = None
    for n in range(1, N + 1):
        z = z*z + c; zd[n] = complex(z)
        if abs(zd[n]) > 1e6: esc = n; break
    M = esc or N
    zd = zd[:M + 1]
    print(f"orbit: {M} steps at {mp.mp.dps} digits{' (escaped!)' if esc else ''} in {time.time() - t0:.1f} s;"
          f" min |z| record returns: ", end='')
    best = np.inf; recs = []
    for n in range(1, M + 1):
        a = abs(zd[n])
        if a < best: best = a; recs.append((n, a))
    print(' '.join(f"{n}" for n, a in recs if n > 2)[-200:])
    t1 = time.time(); cover = np.zeros(M + 1, bool); found = []
    # candidate cycle lengths: all short ones, plus record-return periods and small multiples
    cands = set(range(1, min(RMAX, M//2)))
    for n, _ in recs:
        for k in (1, 2, 3, 4):
            if 2 < n*k < min(M//2, RCAP): cands.add(n*k)
    for r in sorted(cands):
        close = np.abs(zd[r:] - zd[:-r]) < TOL
        d = np.diff(np.concatenate([[0], close.astype(np.int8), [0]]))
        s, e = np.nonzero(d == 1)[0], np.nonzero(d == -1)[0]
        keep = (e - s) >= MINLAPS*r
        for a, b in zip(s[keep], e[keep]):
            lam = float(np.exp(np.sum(np.log(np.abs(2*zd[a:a + r]) + 1e-300))))
            if lam > 1.0:
                found.append((b - a, a, r, (b - a)/r, lam)); cover[a:b] = True
    found.sort(reverse=True)
    print(f"cycle-dwell scan r <= {RMAX}: {time.time() - t1:.1f} s")
    print(f"steps inside repelling-cycle dwells of >= {MINLAPS:g} laps: {cover.sum()} of {M} = {cover.mean():.1%}")
    print("largest dwells (steps, start, cycle r, laps, |lambda|/lap):")
    for f in found[:12]:
        print(f"  {f[0]:7d} from {f[1]:7d}  r {f[2]:5d}  laps {f[3]:8.1f}  |lambda| {f[4]:.3g}")


if __name__ == '__main__':
    main()
