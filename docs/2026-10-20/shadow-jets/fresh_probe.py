#!/usr/bin/env python3
"""Fresh falsification points declared after the order-growth rule was selected."""
import argparse,hashlib,json,time
from pathlib import Path
import mpmath as mp
from shadow_probe import HERE
from tournament import previous

POINTS=[(r,i) for r in ['0.31','0.43','0.57','0.69'] for i in ['-0.00002','0.00002']]


def freeze(path,dps):
    started=time.perf_counter();rows=[]
    with mp.workdps(dps):
        for xr,xi in POINTS:
            eps=mp.mpf('1e-8')*mp.mpc(xr,xi);n,escaped,z,d=previous.direct(mp.mpf('.25')+eps,100000)
            rows.append(dict(x=[xr,xi],n=n,escaped=escaped,z=[mp.nstr(z.real,45),mp.nstr(z.imag,45)],q=[mp.nstr((d*mp.mpf('1e-8')).real,45),mp.nstr((d*mp.mpf('1e-8')).imag,45)]))
            print(xr,xi,n,escaped,flush=True)
    path.write_text(json.dumps(dict(points=POINTS,rows=rows,limit=100000,dps=dps,oracle_seconds=time.perf_counter()-started),indent=2)+'\n')


def export(path,out):
    rows=json.loads(path.read_text())['rows'];lines=[str(len(rows))]
    for r in rows:
        x=complex(*map(float,r['x']))
        lines.append(' '.join(map(str,[x.real*1e-8,x.imag*1e-8,r['n'],*r['z'],*r['q']])))
    out.write_text('\n'.join(lines)+'\n')


def audit(path,other):
    a=json.loads(path.read_text());b=json.loads(other.read_text());assert len(a['rows'])==len(b['rows'])==8
    with mp.workdps(70):
        for x,y in zip(a['rows'],b['rows'],strict=True):
            assert (x['x'],x['n'],x['escaped'])==(y['x'],y['n'],y['escaped'])
            for key in ['z','q']:
                xx=mp.mpc(*x[key]);yy=mp.mpc(*y[key]);assert abs(xx-yy)<mp.mpf('1e-30')*max(abs(xx),mp.mpf('1e-100'))
    (HERE/'results/fresh-audit.json').write_text(json.dumps(dict(points=8,original_dps=a['dps'],audit_dps=b['dps'],relative_digits=30,original_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),audit_sha256=hashlib.sha256(other.read_bytes()).hexdigest()),indent=2)+'\n')
    print('Fresh 100-vs-150-digit direct truth agrees at 8/8 points to 30 relative digits')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('mode',choices=['freeze','export','audit']);p.add_argument('--truth',type=Path,default=HERE/'fresh-truth.json');p.add_argument('--dps',type=int,default=100);p.add_argument('--out',type=Path,default=HERE/'results/fresh-points.txt');p.add_argument('--other',type=Path);a=p.parse_args()
    if a.mode=='freeze':freeze(a.truth,a.dps)
    elif a.mode=='export':export(a.truth,a.out)
    else:audit(a.truth,a.other)
