#!/usr/bin/env python3
"""Fast research probes. Snapshot screening is diagnostic, not safe skip admission."""
import argparse, csv, hashlib, importlib.util, json, math, platform, time
from pathlib import Path
import mpmath as mp
import numpy as np

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('previous', HERE.parent/'series_probe.py')
previous=importlib.util.module_from_spec(spec); spec.loader.exec_module(previous)
SCENES={s[0]:s for s in previous.SCENES}
CHECKS={'cusp-8':[3000,9000,15000,25000,30000],
        'v0-14':[250,322,350,375,390], 'v0-29':[736,813,900,950,975],
        'valley-14':[1000,1800,2000,2400,3000]}
OFFSETS=[('0.4','0'),('0.8','0'),('-0.6','0.45'),('0.4','0.2'),('0.15','-0.8')]


def save_csv(path, rows):
    with path.open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)


def freeze(path,dps=100):
    """Freeze direct independent pixel orbits at predeclared checkpoints."""
    rows=[]
    with mp.workdps(dps):
        for name,checks in CHECKS.items():
            _,re,im,w=SCENES[name]; C=mp.mpc(re,im); rho=mp.mpf(w)
            for xr,xi in OFFSETS:
                c=C+rho*mp.mpc(xr,xi); z=mp.mpc(0); q=mp.mpc(0)
                for n in range(1,max(checks)+1):
                    q=2*z*q+rho; z=z*z+c
                    if n in checks:
                        rows.append(dict(scene=name,n=n,x=[xr,xi],z=[mp.nstr(z.real,45),mp.nstr(z.imag,45)],q=[mp.nstr(q.real,45),mp.nstr(q.imag,45)]))
                    if abs(z)>2: break
            print('freeze',name,flush=True)
    path.write_text(json.dumps(dict(dps=dps,checks=CHECKS,offsets=OFFSETS,scenes=list(SCENES.values()),rows=rows),indent=2)+'\n')


def pade(b,m):
    """Scaled linear solve for a [p-m/m] approximant of a normalized jet."""
    p=len(b)-1; L=p-m
    if not m:return b.copy(),np.array([1+0j]),1.
    A=np.array([[b[k-j] if k-j>=0 else 0j for j in range(1,m+1)] for k in range(L+1,p+1)])
    rhs=-b[L+1:p+1]
    scale=np.maximum(np.max(np.abs(A),axis=1),np.abs(rhs))
    if np.any(scale==0):raise np.linalg.LinAlgError('degenerate')
    A=A/scale[:,None]; rhs=rhs/scale
    cond=float(np.linalg.cond(A)); Q=np.r_[1+0j,np.linalg.solve(A,rhs)]
    P=np.convolve(b,Q)[:L+1]
    return P,Q,cond


def value(P,Q,x):
    pv=np.polynomial.polynomial.polyval(x,P); qv=np.polynomial.polynomial.polyval(x,Q)
    dp=np.polynomial.polynomial.polyval(x,np.arange(1,len(P))*P[1:]) if len(P)>1 else 0j
    dq=np.polynomial.polynomial.polyval(x,np.arange(1,len(Q))*Q[1:]) if len(Q)>1 else 0j
    return pv/qv,(dp*qv-pv*dq)/(qv*qv),abs(qv)


def jets(name,order,checks):
    """Shared reference and normalized Taylor coefficients, recorded at checkpoints."""
    _,re,im,w=SCENES[name]; rho=float(w)
    with mp.workdps(100):ref=previous.reference(mp.mpc(re,im),max(checks))
    b=np.zeros(order+1,dtype=complex); snapshots={}
    for n in range(1,len(ref)):
        b=2*ref[n-1]*b+np.convolve(b,b)[:order+1];b[1]+=rho
        if n in checks:snapshots[n]=(b.copy(),ref[n])
    return snapshots


def screen(truth_path,out):
    """Same jet, polynomial vs balanced/near-balanced rational approximants."""
    started=time.perf_counter();truth=json.loads(truth_path.read_text());rows=[]
    for name,checks in CHECKS.items():
        for order in [4,8,12]:
            t=time.perf_counter();snapshots=jets(name,order,checks);setup=time.perf_counter()-t
            for n,(b,Z) in snapshots.items():
                targets=[r for r in truth['rows'] if r['scene']==name and r['n']==n]
                for m in range(order//2+1):
                    try:P,Q,cond=pade(b,m)
                    except np.linalg.LinAlgError:continue
                    lower=abs(Q[0])-sum(abs(Q[1:])); errs=[];ders=[];denoms=[]
                    for target in targets:
                        x=complex(*map(float,target['x'])); zt=complex(*map(float,target['z']));qt=complex(*map(float,target['q']))
                        u,q,den=value(P,Q,x);errs.append(abs(Z+u-zt));ders.append(abs(q-qt)/max(abs(qt),1e-300));denoms.append(den)
                    if not errs:continue
                    # Evaluation microbenchmark includes both state and derivative, same implementation.
                    xs=[complex(*map(float,r['x'])) for r in targets]
                    t=time.perf_counter()
                    for _ in range(50):
                        for x in xs:value(P,Q,x)
                    eval_s=(time.perf_counter()-t)/(50*len(xs))
                    rows.append(dict(scene=name,n=n,order=order,denom_degree=m,pixels=len(errs),
                        state_error=max(errs),derivative_relative_error=max(ders),
                        pass_accuracy=max(errs)<1e-10 and max(ders)<1e-6,
                        denominator_lower=lower,sampled_min_denominator=min(denoms),solve_condition=cond,
                        jet_setup_s=setup,evaluate_s=eval_s))
            print('screen',name,order,flush=True)
    out.mkdir(parents=True,exist_ok=True);save_csv(out/'screen.csv',rows)
    (out/'screen-run.json').write_text(json.dumps(dict(elapsed_s=time.perf_counter()-started,truth_sha256=hashlib.sha256(truth_path.read_bytes()).hexdigest(),numpy=np.__version__,mpmath=mp.__version__,python=platform.python_version()),indent=2)+'\n')
    print('screen seconds',time.perf_counter()-started)


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['freeze','screen']);p.add_argument('--truth',type=Path,default=HERE/'snapshots.json');p.add_argument('--out',type=Path,default=HERE/'results');p.add_argument('--dps',type=int,default=100)
    a=p.parse_args()
    if a.mode=='freeze':freeze(a.truth,a.dps)
    else:screen(a.truth,a.out)
