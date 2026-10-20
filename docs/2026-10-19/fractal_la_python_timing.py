#!/usr/bin/env python3
"""Selected-case Python evaluator timing, excluding per-scene preprocessing.
Not native wall-clock proof. Run: python fractal_la_python_timing.py
"""
import sys,time,statistics,csv
from pathlib import Path
import mpmath as mp
sys.path.insert(0,str(Path(__file__).parent))
from fractal_la_experiment import N, CASES, OFFSETS, hp_reference, build_bla, build_jets, CertTable, plain, bla, staged, initial_series, EscapeGuard
mp.mp.dps=100
results=[]
for name in ['attracting_-1','seahorse_edge','misiurewicz_i']:
    re,im=CASES[name]; hp=hp_reference(mp.mpc(mp.mpf(re),mp.mpf(im)),N)
    ref=[complex(z) for z in hp]; rho=1e-20
    jb=build_jets(ref,4)[0]; b=build_bla(ref,rho,eta=.05)[0]
    t=time.perf_counter(); cert=CertTable(jb,ref,rho); prep=time.perf_counter()-t
    dcs=[complex(ox,oy)*rho for ox,oy in OFFSETS]
    methods={
       'plain':lambda dc:plain(ref,dc,EscapeGuard(hp,ref)),
       'BLA':lambda dc:bla(ref,dc,b,EscapeGuard(hp,ref)),
       'stage2':lambda dc:staged(ref,dc,jb,rho,cert,EscapeGuard(hp,ref)),
       'series4_direct':lambda dc:initial_series(ref,dc,rho),
    }
    for method,fn in methods.items():
        # 300 repeats * 3 sample pixels per case
        times=[]
        for trial in range(3):
            t=time.perf_counter()
            for k in range(300):
                for dc in dcs: fn(dc)
            times.append((time.perf_counter()-t)/(900))
        results.append(dict(case=name,method=method,us_per_pixel=1e6*statistics.median(times),stage_cert_preprocess_s=prep))
        print(name, method, 'median microsec/pixel',round(1e6*statistics.median(times),2),flush=True)
with open('/mnt/data/fractal_la_python_timing.csv','w',newline='') as f:
    ww=csv.DictWriter(f,fieldnames=list(results[0]));ww.writeheader();ww.writerows(results)
