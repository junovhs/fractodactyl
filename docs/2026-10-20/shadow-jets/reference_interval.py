#!/usr/bin/env python3
"""Outward-rounded REAL reference-only enclosure; not a chart/pixel certificate."""
import json,math,time
import mpmath as mp
from shadow_probe import HERE
from tournament import save_csv


def down(x):return math.nextafter(x,-math.inf)
def up(x):return math.nextafter(x,math.inf)
def step_lower(x,e):return down(down(x+down(x*x))+e)
def step_upper(x,e):return up(up(x+up(x*x))+e)


def run():
    started=time.perf_counter();lo=hi=w=-.5;eps=5e-9;el=down(eps);eh=up(eps);rows=[]
    with mp.workdps(100):
        exact=mp.mpf('-.5');ep=mp.mpf('5e-9')
        assert mp.mpf(el)<=ep<=mp.mpf(eh)
        for n in range(1,36001):
            assert lo>=-.5
            lo=step_lower(lo,el);hi=step_upper(hi,eh);w=w+w*w+eps;exact=exact+exact*exact+ep
            assert mp.mpf(lo)<=exact<=mp.mpf(hi)
            if n%512==0 or n in [21888,26016,31104,31968,34720,36000]:
                rows.append(dict(n=n,lower=lo,upper=hi,centered_reference=w,reference_radius=max(abs(w-lo),abs(hi-w)),actual_error=float(abs(mp.mpf(w)-exact))))
    save_csv(HERE/'results/reference-interval.csv',rows)
    (HERE/'results/reference-interval-run.json').write_text(json.dumps(dict(elapsed_s=time.perf_counter()-started,checks=36000,scope='Real centered reference only; excludes coefficients, parameter regions and per-pixel rounding'),indent=2)+'\n')
    for r in rows:
        if r['n'] in [21888,26016,31104,31968,34720,36000]:print(r)


if __name__=='__main__':run()
