#!/usr/bin/env python3
"""Shadow Taylor jet bounds a low-order rational chart; research, no rounded certificate."""
import argparse,csv,importlib.util,json,math,sys,time
from pathlib import Path
import mpmath as mp
import numpy as np

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'rational-charts'))
from guard_probe import onepole,norm,dnorm,add
from local_probe import ref_pair,evaluate
from tournament import save_csv


def envelope(b,E,F):
    """Ideal arithmetic enclosure of polynomial + remainder."""
    return norm(b)+E,dnorm(b)+F


def comparison(b,E,F,p,rational):
    if p==len(b)-1 and not rational:return b.copy(),np.array([1+0j]),E,F,1.
    if rational and abs(b[p-1])>1e-300:P,Q,_=onepole(b[:p+1])
    else:P,Q=b[:p+1].copy(),np.array([1+0j])
    q=abs(Q[0])-norm(Q[1:])
    if q<=0:return P,Q,math.inf,math.inf,q
    H=add(np.convolve(b,Q),-P)
    err=norm(H)/q+E
    der=dnorm(H)/q+norm(H)*dnorm(Q)/q**2+F
    return P,Q,err,der,q


def g_lower(b,Z,eps,rho,E):
    """Lower bound for |w²+epsilon| on the unit parameter disk, ideal arithmetic."""
    W=b.copy();W[0]+=Z-.5
    G=np.convolve(W,W);G[0]+=eps;G[1]+=rho
    return max(0.,abs(G[0])-norm(G[1:])-2*norm(W)*E-E*E)


def build(K,scale,maxn,phase=1e-6,stride=32):
    started=time.perf_counter();rho=scale*1e-8;eps=.5e-8
    with mp.workdps(100):ref,ws=ref_pair(mp.mpf('.25')+mp.mpf('.5')*mp.mpf('1e-8'),maxn)
    reference_s=time.perf_counter()-started;t=time.perf_counter()
    b=np.zeros(K+1,dtype=complex);E=F=0.
    configs=[(p,r,rule) for p in [8,12,16,24,K] if p<=K for r in [False,True] if not(r and p==K) for rule in ['fixed','phase-ref','phase-disk','phase-strip']]
    configs=list(dict.fromkeys(configs));accepted={};history=[];stop='limit'
    for n in range(1,len(ref)):
        M=norm(b);D=dnorm(b);square=np.convolve(b,b)
        tail=norm(square[K+1:]);tail_d=float(np.sum(np.arange(K+1,len(square))*np.abs(square[K+1:])))
        E2=2*(abs(ref[n-1])+M)*E+E*E+tail
        F2=2*(abs(ref[n-1])+M+E)*F+2*E*D+tail_d
        b=2*ref[n-1]*b+square[:K+1];b[1]+=rho;E,F=E2,F2
        Mnext,Dnext=envelope(b,E,F)
        if not all(map(math.isfinite,[Mnext,Dnext])):stop='nonfinite';break
        if abs(ref[n])+Mnext>=2:stop='shadow-bailout';break
        if n%stride:continue
        phase_ref=abs(ws[n]*ws[n]+eps)
        phase_disk=g_lower(b,ref[n],eps,rho,E)
        # Actual training rectangle: global Re(x) in [.3,.7], |Im(x)|<=1e-5.
        # On its real axis W(s)^2+epsilon(s)>=3e-9. Bound vertical variation.
        W=b.copy();W[0]+=ref[n]-.5
        eta=1e-5/scale
        phase_strip=max(0.,3e-9-eta*(2*norm(W)*dnorm(W)+rho)-2*norm(W)*E-E*E)
        for p,r,rule in configs:
            P,Q,e,f,q= comparison(b,E,F,p,r)
            budget=1e-15 if rule=='fixed' else phase*(phase_ref if rule=='phase-ref' else phase_strip if rule=='phase-strip' else phase_disk)
            if e<=budget and f<=1e-8*max(rho,Dnext) and q>0:
                key=f'p{p}-r{int(r)}-{rule}'
                accepted[key]=dict(name=key,n=n,P=[[v.real,v.imag] for v in P],Q=[[v.real,v.imag] for v in Q],
                    E=e,F=f,phase_ref=phase_ref,phase_disk=phase_disk,budget=budget,w=[ws[n].real,ws[n].imag],
                    shadow_error=E,shadow_derivative_error=F,phase_strip=phase_strip,order=p,rational=r,rule=rule)
        if n%1024==0:history.append(dict(n=n,shadow_error=E,shadow_derivative_error=F,phase_ref=phase_ref,phase_disk=phase_disk))
    return accepted,history,dict(K=K,radius=scale,maxn=maxn,phase_tolerance=phase,stride=stride,reference_s=reference_s,shadow_build_s=time.perf_counter()-t,total_s=time.perf_counter()-started,stop=stop,last_iteration=n)


def test(accepted,scale,truthpath):
    truth=json.loads(truthpath.read_text());rows=[]
    for key,c in accepted.items():
        init=(c['n'],[complex(*v) for v in c['P']],[complex(*v) for v in c['Q']],c['E'],c['F'])
        ws=[0j]*(c['n']+1);ws[c['n']]=complex(*c['w']);bad=0;maxq=maxnu=0.;ordinary=0
        for target in truth['rows']:
            x=complex(*map(float,target['x']));n,escaped,z,q,steps=evaluate(ws,x*1e-8,(x-.5)/scale,scale,init,100000)
            zt=complex(*map(float,target['z']));qt=complex(*map(float,target['q']))
            qe=abs(q-qt)/abs(qt)
            nu=abs(math.log2(math.log(abs(z)))-math.log2(math.log(abs(zt)))) if escaped and target['escaped'] else 0.
            ok=(n==target['n'] and escaped==target['escaped'] and abs(z-zt)<1e-6*max(1,abs(zt)) and qe<1e-6 and nu<1e-6)
            bad+=not ok;maxq=max(maxq,qe);maxnu=max(maxnu,nu);ordinary+=steps
        rows.append(dict(method=key,prefix=c['n'],order=c['order'],rational=c['rational'],rule=c['rule'],errors=bad,pixels=len(truth['rows']),max_derivative_relative_error=maxq,max_smooth_error=maxnu,mean_ordinary=ordinary/len(truth['rows']),state_bound=c['E'],state_budget=c['budget'],phase_disk=c['phase_disk']))
    return rows


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--K',type=int,default=32);p.add_argument('--radius',type=float,default=.21);p.add_argument('--maxn',type=int,default=36000);p.add_argument('--phase',type=float,default=1e-6);p.add_argument('--out',type=Path,default=HERE/'results/K32');a=p.parse_args()
    cs,h,meta=build(a.K,a.radius,a.maxn,a.phase);a.out.mkdir(parents=True,exist_ok=True)
    (a.out/'charts.json').write_text(json.dumps(cs,indent=2)+'\n');(a.out/'run.json').write_text(json.dumps(meta,indent=2)+'\n');save_csv(a.out/'history.csv',h)
    rows=test(cs,a.radius,HERE.parent/'rational-charts/local-truth.json');save_csv(a.out/'training.csv',rows)
    for r in rows:print(r['method'],r['prefix'],'errors',r['errors'])
    print(meta)
