#!/usr/bin/env python3
"""Ideal-arithmetic rational residual guard; floating-point rounding is NOT certified."""
import argparse,csv,json,math,time
from pathlib import Path
import mpmath as mp
import numpy as np
from tournament import HERE,SCENES,previous,pade,value,save_csv


def add(*terms):
    n=max(map(len,terms));r=np.zeros(n,dtype=complex)
    for t in terms:r[:len(t)]+=t
    return r


def norm(p):return float(np.sum(np.abs(p)))
def dnorm(p):return float(np.sum(np.arange(1,len(p))*np.abs(p[1:])))


def onepole(b):
    if abs(b[-2])<1e-300:return b.copy(),np.array([1+0j]),1.
    Q=np.array([1+0j,-b[-1]/b[-2]])
    return np.convolve(b,Q)[:len(b)-1],Q,1.


def residual(P,Q,A,B,Z,rho):
    """(2Z P Q + P² + rho*x*Q²)B - A Q² over Q²B."""
    qlower=abs(Q[0])-norm(Q[1:]);blower=abs(B[0])-norm(B[1:])
    if min(qlower,blower)<=0:return math.inf,math.inf,0.
    Q2=np.convolve(Q,Q)
    H=add(2*Z*np.convolve(P,Q),np.convolve(P,P),rho*np.r_[0j,Q2])
    D=add(np.convolve(H,B),-np.convolve(A,Q2))
    den=qlower*qlower*blower
    e=norm(D)/den
    f=dnorm(D)/den+e*(2*dnorm(Q)/qlower+dnorm(B)/blower)
    return e,f,min(qlower,blower)


def build_guard(ref,rho,order,m,tol=1e-12,dtol=1e-8):
    b=np.zeros(order+1,dtype=complex);P=b.copy();Q=np.array([1+0j]);E=F=0.
    last=(0,P.copy(),Q.copy(),E,F);min_den=1.;maxcond=1.
    counts={'pole':0,'state':0,'derivative':0,'bailout':0,'nonfinite':0}
    for n in range(1,len(ref)):
        b=2*ref[n-1]*b+np.convolve(b,b)[:order+1];b[1]+=rho
        try:
            if m==1 and n>2*order:A,B,cond=onepole(b)
            else:A,B,cond=pade(b,m if n>2*order else 0)
        except np.linalg.LinAlgError:
            A,B,cond=b.copy(),np.array([1+0j]),1.
        qlower=abs(Q[0])-norm(Q[1:]);blower=abs(B[0])-norm(B[1:])
        if min(qlower,blower)<=0: reason='pole';break
        M=norm(P)/qlower;D=dnorm(P)/qlower+norm(P)*dnorm(Q)/qlower**2
        defect,deriv_defect,den=residual(P,Q,A,B,ref[n-1],rho)
        Enext=2*(abs(ref[n-1])+M)*E+E*E+defect
        Fnext=2*(abs(ref[n-1])+M+E)*F+2*E*D+deriv_defect
        Mnext=norm(A)/blower;Dnext=dnorm(A)/blower+norm(A)*dnorm(B)/blower**2
        if not all(map(math.isfinite,[Enext,Fnext,Mnext,Dnext])):reason='nonfinite';break
        if abs(ref[n])+Mnext+Enext>=2:reason='bailout';break
        if Enext>tol:reason='state';break
        if Fnext>dtol*max(rho,Dnext):reason='derivative';break
        P,Q,E,F=A,B,Enext,Fnext
        min_den=min(min_den,den);maxcond=max(maxcond,cond)
        last=(n,P.copy(),Q.copy(),E,F)
    else:reason='limit'
    return last,reason,min_den,maxcond


def run(scene,order,denom,maxn,tol,out):
    started=time.perf_counter();_,re,im,w=SCENES[scene];rho=float(w)
    with mp.workdps(100):ref=previous.reference(mp.mpc(re,im),maxn)
    ref_s=time.perf_counter()-started
    t=time.perf_counter();init,reason,den,cond=build_guard(ref,rho,order,denom,tol)
    setup=time.perf_counter()-t
    row=dict(scene=scene,order=order,denom=denom,tolerance=tol,prefix=init[0],stop=reason,
             state_bound=init[3],derivative_bound=init[4],min_denominator=den,max_solve_condition=cond,reference_s=ref_s,setup_s=setup,total_s=time.perf_counter()-started)
    out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(row,indent=2)+'\n');print(json.dumps(row),flush=True)
    return row


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--scene',choices=SCENES,default='cusp-8');p.add_argument('--order',type=int,default=8);p.add_argument('--denom',type=int,default=1);p.add_argument('--maxn',type=int,default=30000);p.add_argument('--tol',type=float,default=1e-12);p.add_argument('--out',type=Path,default=HERE/'results/guard.json');a=p.parse_args()
    run(a.scene,a.order,a.denom,a.maxn,a.tol,a.out)
