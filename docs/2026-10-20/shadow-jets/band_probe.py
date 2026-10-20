#!/usr/bin/env python3
"""Fast cross-family test of shadow vs compressed jets on existing frozen endpoints."""
import argparse,json,math,sys,time
import mpmath as mp
import numpy as np
from shadow_probe import HERE,comparison
from tournament import SCENES,previous,save_csv
from endpoint_probe import evaluate
from guard_probe import norm,dnorm


def run(scene,K,out):
    started=time.perf_counter();limits={'v0-14':550,'v0-29':1200,'valley-14':4000}
    _,re,im,width=SCENES[scene];rho=float(width)
    with mp.workdps(100):ref=previous.reference(mp.mpc(re,im),100000)
    reference_s=time.perf_counter()-started
    b=np.zeros(K+1,dtype=complex);E=F=0.;cs={};stop='limit';t=time.perf_counter()
    for n in range(1,min(len(ref),limits[scene]+1)):
        square=np.convolve(b,b);M=norm(b);D=dnorm(b)
        tail=norm(square[K+1:]);td=float(np.sum(np.arange(K+1,len(square))*np.abs(square[K+1:])))
        E2=2*(abs(ref[n-1])+M)*E+E*E+tail;F2=2*(abs(ref[n-1])+M+E)*F+2*E*D+td
        b=2*ref[n-1]*b+square[:K+1];b[1]+=rho;E,F=E2,F2
        if not math.isfinite(E) or abs(ref[n])+norm(b)+E>=2:stop='shadow-bailout';break
        for p,r in [(8,False),(8,True),(16,True),(K,False)]:
            P,Q,e,f,q= comparison(b,E,F,p,r)
            if e<=1e-15 and f<=1e-8*max(rho,dnorm(b)+F) and q>0:
                cs[f'p{p}-r{int(r)}']=(n,list(map(complex,P)),list(map(complex,Q)),e,f)
    setup=time.perf_counter()-t;targets=[r for r in json.loads((HERE.parent/'truth.json').read_text())['rows'] if r['scene']==scene];rows=[]
    for key,init in cs.items():
        bad=0;maxq=maxnu=0.;steps=0
        for target in targets:
            x=complex(*map(float,target['x']));n,escaped,z,q,ordinary,supported=evaluate(ref,rho,x,init,100000)
            zt=complex(*map(float,target['z']));qt=complex(*map(float,target['q']))
            qe=abs(q-qt)/abs(qt);nu=abs(math.log2(math.log(abs(z)))-math.log2(math.log(abs(zt)))) if escaped and target['escaped'] else 0.
            ok=supported and n==target['n'] and escaped==target['escaped'] and abs(z-zt)<=1e-6*max(1,abs(zt)) and qe<=1e-6 and nu<=1e-6
            bad+=not ok;maxq=max(maxq,qe);maxnu=max(maxnu,nu);steps+=ordinary
        rows.append(dict(scene=scene,K=K,method=key,prefix=init[0],errors=bad,pixels=len(targets),max_derivative_relative_error=maxq,max_smooth_error=maxnu,mean_ordinary=steps/len(targets),reference_s=reference_s,shadow_build_s=setup,stop=stop))
    out.parent.mkdir(parents=True,exist_ok=True);save_csv(out,rows)
    for r in rows:print(r)
    print('elapsed',time.perf_counter()-started)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--scene',choices=['v0-14','v0-29','valley-14'],default='v0-29');p.add_argument('--K',type=int,default=64);p.add_argument('--out',type=__import__('pathlib').Path,default=HERE/'results/band.csv');a=p.parse_args();run(a.scene,a.K,a.out)
