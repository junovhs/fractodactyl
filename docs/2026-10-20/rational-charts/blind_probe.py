#!/usr/bin/env python3
"""Holdout orbit samples; reuse the already frozen local charts without refitting."""
import argparse,json,time
from pathlib import Path
import mpmath as mp
from tournament import HERE,previous

POINTS=[(r,i) for r in ['0.32','0.39','0.48','0.59','0.68'] for i in ['-0.00003','0.00003']]+[('0.5','-0.1'),('0.5','0.1')]


def freeze(path):
    started=time.perf_counter();rows=[]
    with mp.workdps(100):
        for xr,xi in POINTS:
            eps=mp.mpf('1e-8')*mp.mpc(xr,xi);n,escaped,z,d=previous.direct(mp.mpf('.25')+eps,100000)
            rows.append(dict(x=[xr,xi],n=n,escaped=escaped,z=[mp.nstr(z.real,45),mp.nstr(z.imag,45)],q=[mp.nstr((d*mp.mpf('1e-8')).real,45),mp.nstr((d*mp.mpf('1e-8')).imag,45)]))
            print('holdout',xr,xi,n,escaped,flush=True)
    path.write_text(json.dumps(dict(points=POINTS,rows=rows,limit=100000,dps=100,oracle_seconds=time.perf_counter()-started),indent=2)+'\n')


def export(truth,charts,out):
    source=charts.read_text().splitlines();np=int(source[0]);body=source[1+np:]
    rows=json.loads(truth.read_text())['rows'];lines=[str(len(rows))]
    for r in rows:
        x=complex(*map(float,r['x']))
        lines.append(' '.join(map(str,[x.real*1e-8,x.imag*1e-8,r['n'],*r['z'],*r['q']])))
    out.write_text('\n'.join(lines+body)+'\n')


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['freeze','export']);p.add_argument('--truth',type=Path,default=HERE/'holdout-truth.json');p.add_argument('--charts',type=Path,default=HERE/'results/fair-local/charts.txt');p.add_argument('--out',type=Path,default=HERE/'results/holdout-charts.txt');a=p.parse_args()
    if a.mode=='freeze':freeze(a.truth)
    else:export(a.truth,a.charts,a.out)
