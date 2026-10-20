#!/usr/bin/env python3
"""End-to-end sparse test: guarded prefix, ordinary perturbation tail, frozen truth."""
import argparse,csv,json,math,statistics,time
from pathlib import Path
import mpmath as mp
from tournament import HERE,SCENES,previous,save_csv
from guard_probe import build_guard


def horner(p,x):
    v=0j;d=0j
    for a in reversed(p):d=d*x+v;v=v*x+a
    return v,d


def evaluate(ref,rho,x,init,limit):
    n,P,Q,E,F=init
    pv,pd=horner(P,x);qv,qd=horner(Q,x)
    u=pv/qv;q=(pd*qv-pv*qd)/(qv*qv);ordinary=0
    while n<limit and abs(ref[n]+u)<=2:
        if n+1>=len(ref):return n,False,complex('nan'),complex('nan'),ordinary,False
        q=2*(ref[n]+u)*q+rho;u=u*(2*ref[n]+u)+rho*x;n+=1;ordinary+=1
    z=ref[n]+u
    return n,abs(z)>2,z,q,ordinary,True


def run(scene,configs,out,limit=100000):
    started=time.perf_counter();truth=json.loads((HERE.parent/'truth.json').read_text());assert limit==truth['limit']
    targets=[r for r in truth['rows'] if r['scene']==scene]
    _,re,im,w=SCENES[scene];rho=float(w)
    with mp.workdps(100):ref=previous.reference(mp.mpc(re,im),limit)
    ref_s=time.perf_counter()-started;rows=[];pixels=[];base=None
    for order,m,tol in configs:
        t=time.perf_counter()
        if order:init,stop,den,cond=build_guard(ref,rho,order,m,tol)
        else:init=(0,[0j],[1+0j],0.,0.);stop='plain';den=cond=1.
        setup=time.perf_counter()-t if order else 0.
        init=(init[0],list(map(complex,init[1])),list(map(complex,init[2])),init[3],init[4])
        xs=[complex(*map(float,r['x'])) for r in targets]
        times=[]
        for _ in range(3):
            t=time.perf_counter();results=[evaluate(ref,rho,x,init,limit) for x in xs];times.append((time.perf_counter()-t)/len(xs))
        per=statistics.median(times)
        if not order:base=per
        errors=0;escerrors=0
        for target,res in zip(targets,results):
            n,escaped,z,q,ordinary,supported=res;same=supported and n==target['n'] and escaped==target['escaped']
            zt=complex(*map(float,target['z']));qt=complex(*map(float,target['q']))
            ze=abs(z-zt) if same else math.inf;qe=abs(q-qt)/max(abs(qt),1e-300) if same else math.inf
            nu=abs(math.log2(math.log(abs(z)))-math.log2(math.log(abs(zt)))) if same and escaped else 0. if same else math.inf
            ok=same and ze<=1e-6*max(1,abs(zt)) and qe<=1e-6 and nu<=1e-6;errors+=not ok;escerrors+=not same
            pixels.append(dict(scene=scene,order=order,denom=m,tolerance=tol,x=str(target['x']),n=n,truth_n=target['n'],escaped=escaped,truth_escaped=target['escaped'],ordinary=ordinary,correct=ok,state_error=ze,derivative_relative_error=qe,smooth_error=nu))
        delta=base-per
        rows.append(dict(scene=scene,order=order,denom=m,tolerance=tol,prefix=init[0],stop=stop,errors=errors,escape_errors=escerrors,setup_s=setup,reference_s=ref_s,pixel_s=per,break_even=math.ceil(setup/delta) if delta>0 else 'never',warm_speedup=base/per,cold_three_pixel_speedup=(ref_s+3*base)/(ref_s+setup+3*per),score=0 if errors else (ref_s+3*base)/(ref_s+setup+3*per)))
        print(scene,order,m,tol,'prefix',init[0],'errors',errors,'warm',round(base/per,3),'setup',round(setup,3),flush=True)
    out.mkdir(parents=True,exist_ok=True);save_csv(out/'endpoints.csv',pixels);save_csv(out/'timings.csv',rows)
    (out/'run.json').write_text(json.dumps(dict(elapsed_s=time.perf_counter()-started,note='Sparse Python evaluator only; no native/full-frame speedup',configs=configs),indent=2)+'\n')


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--scene',choices=SCENES,default='cusp-8');p.add_argument('--out',type=Path,default=HERE/'results/endpoints');a=p.parse_args()
    run(a.scene,[(0,0,1e-12),(8,0,1e-12),(8,1,1e-12),(12,1,1e-12),(12,1,1e-18)],a.out)
