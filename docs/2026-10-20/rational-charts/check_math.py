#!/usr/bin/env python3
"""Algebra, frozen-truth audit, and preservation of positive/negative controls."""
import argparse,csv,hashlib,json,math
import mpmath as mp
import numpy as np
from tournament import HERE,pade,value
from guard_probe import onepole,residual

rng=np.random.default_rng(731)
for _ in range(100):
    P=(rng.normal(size=4)+1j*rng.normal(size=4))*.02
    A=(rng.normal(size=4)+1j*rng.normal(size=4))*.02
    Q=np.r_[1+0j,(rng.normal(size=2)+1j*rng.normal(size=2))*.03]
    B=np.r_[1+0j,(rng.normal(size=2)+1j*rng.normal(size=2))*.03]
    Z=complex(*rng.normal(size=2))*.5;rho=.001
    E,F,den=residual(P,Q,A,B,Z,rho)
    for k in range(8):
        x=.9*np.exp(2j*np.pi*k/8);r,dr,_=value(P,Q,x);s,ds,_=value(A,B,x)
        defect=2*Z*r+r*r+rho*x-s
        derivative=2*(Z+r)*dr+rho-ds
        assert abs(defect)<=E+1e-12 and abs(derivative)<=F+1e-12
for order in [4,8,12,16]:
    b=np.r_[0j,(rng.normal(size=order)+1j*rng.normal(size=order))*.01]
    P,Q,_=onepole(b)
    reconstructed=[]
    for k in range(order+1):
        reconstructed.append((P[k] if k<len(P) else 0j)-(Q[1]*reconstructed[k-1] if k else 0j))
    assert np.max(np.abs(np.array(reconstructed)-b))<1e-12
print('800 complex residual/derivative bound checks; four one-pole jet identities passed')

rows=list(csv.DictReader((HERE/'results/fair-local/local-timings.csv').open()))
assert all(r['errors']=='0' for r in rows)
radius=list(csv.DictReader((HERE/'results/radius/local-timings.csv').open()))
assert all(r['errors']=='6' for r in radius if r['method']!='plain')
precision=list(csv.DictReader((HERE/'results/chart-precision.csv').open()))
assert all(r['errors']=='6' for r in precision)
native=list(csv.DictReader((HERE/'results/native-holdout.csv').open()))
assert all(r['failures']=='0' for r in native if r['method']!='local-poly8')
assert next(r for r in native if r['method']=='local-poly8')['failures']=='2'
print('Passing rational charts and rejected narrow/holdout variants remain recorded')

p=argparse.ArgumentParser();p.add_argument('--audit',type=str);a=p.parse_args()
if a.audit:
    original=(HERE/'local-truth.json').read_bytes();audited=open(a.audit,'rb').read()
    t=json.loads(original);v=json.loads(audited)
    assert len(t['rows'])==len(v['rows'])==9
    worst=mp.mpf(0)
    with mp.workdps(70):
        for x,y in zip(t['rows'],v['rows'],strict=True):
            assert (x['x'],x['n'],x['escaped'])==(y['x'],y['n'],y['escaped'])
            for key in ['z','q']:
                xx=mp.mpc(*x[key]);yy=mp.mpc(*y[key]);err=abs(xx-yy)/max(abs(xx),mp.mpf('1e-100'));worst=max(worst,err);assert err<mp.mpf('1e-30')
    report=dict(points=9,original_dps=t['dps'],audit_dps=v['dps'],max_relative_error=str(worst),original_sha256=hashlib.sha256(original).hexdigest(),audit_sha256=hashlib.sha256(audited).hexdigest())
    (HERE/'results/truth-audit.json').write_text(json.dumps(report,indent=2)+'\n')
    print('100-vs-150-digit direct truth audit: 9/9 agree to 30 relative decimal digits')
