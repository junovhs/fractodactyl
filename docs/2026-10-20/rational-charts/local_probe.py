#!/usr/bin/env python3
"""Local parameter charts through the cusp, with centered ordinary continuation.
All arms use w=z-1/2, w_next=w+w²+epsilon for their ordinary tail.
"""
import argparse,csv,json,math,statistics,time
from pathlib import Path
import mpmath as mp
import numpy as np
from tournament import HERE,save_csv,previous
from guard_probe import build_guard
from endpoint_probe import horner

POINTS=[(r,i) for r in ['0.3','0.5','0.7'] for i in ['-0.00001','0','0.00001']]


def freeze(path,dps=100):
    rows=[]
    with mp.workdps(dps):
        for xr,xi in POINTS:
            eps=mp.mpf('1e-8')*mp.mpc(xr,xi)
            n,escaped,z,d=previous.direct(mp.mpf('.25')+eps,100000)
            rows.append(dict(x=[xr,xi],n=n,escaped=escaped,z=[mp.nstr(z.real,45),mp.nstr(z.imag,45)],q=[mp.nstr((d*mp.mpf('1e-8')).real,45),mp.nstr((d*mp.mpf('1e-8')).imag,45)]))
            print('local truth',xr,xi,n,flush=True)
    path.write_text(json.dumps(dict(points=POINTS,rows=rows,limit=100000,dps=dps),indent=2)+'\n')


def ref_pair(center,limit):
    z=mp.mpc(0);ref=[0j];ws=[-.5+0j]
    for _ in range(limit):
        z=z*z+center;ref.append(complex(z));ws.append(complex(z-mp.mpf('.5')))
        if abs(z)>4:break
    return ref,ws


def evaluate(ws,eps,x,scale,init,limit):
    n,P,Q,E,F=init;pv,pd=horner(P,x);qv,qd=horner(Q,x)
    w=ws[n]+pv/qv
    q=(pd*qv-pv*qd)/(qv*qv)/scale # normalized by global rho, rather than local rho
    ordinary=0
    while n<limit and abs(w+.5)<=2:
        q=(1+2*w)*q+1e-8;w=w+w*w+eps;n+=1;ordinary+=1
    return n,abs(w+.5)>2,w+.5,q,ordinary


def run(truthpath,out,family="initial",tol=1e-12):
    started=time.perf_counter();truth=json.loads(truthpath.read_text());rows=[];pixels=[];baseline=None
    configs=[('plain',0.,1.,0,0),('global-poly8',0.,1.,8,0),('global-pole12',0.,1.,12,1),
             ('local-poly8',.5,.25,8,0),('local-pole8',.5,.25,8,1),('local-pole12',.5,.25,12,1)]
    if family=="fair-local":
        configs=[("plain",0.,1.,0,0),("local-poly8",.5,.25,8,0),("local-poly12",.5,.25,12,0),("local-pole12",.5,.25,12,1),("local-pole16",.5,.25,16,1)]
    elif family=="radius":
        configs=[("plain",0.,1.,0,0),("local21-poly12",.5,.21,12,0),("local21-pole12",.5,.21,12,1),("local21-pole16",.5,.21,16,1)]
    if family=="budget":
        configs=[("plain",0.,1.,0,0),("budget-poly16",.5,.21,16,0),("budget-pole16",.5,.21,16,1)]
    cache={}
    exported=[]
    for name,center,scale,order,m in configs:
        ref_s=0.
        if (center,scale) not in cache:
            t=time.perf_counter()
            with mp.workdps(100):cache[center,scale]=ref_pair(mp.mpf('.25')+mp.mpf(str(center))*mp.mpf('1e-8'),100000)
            ref_s=time.perf_counter()-t
            cache[center,scale]+=(ref_s,)
        ref,ws,ref_s=cache[center,scale]
        t=time.perf_counter()
        if order:init,stop,den,cond=build_guard(ref,scale*1e-8,order,m,tol)
        else:init=(0,[0j],[1+0j],0.,0.);stop='plain'
        setup=time.perf_counter()-t if order else 0.
        init=(init[0],list(map(complex,init[1])),list(map(complex,init[2])),init[3],init[4])
        args=[(complex(*map(float,r['x']))*1e-8,(complex(*map(float,r['x']))-center)/scale) for r in truth['rows']]
        times=[]
        for _ in range(3):
            t=time.perf_counter();results=[evaluate(ws,eps,x,scale,init,100000) for eps,x in args];times.append((time.perf_counter()-t)/len(args))
        per=statistics.median(times)
        if baseline is None:baseline=per
        errors=0;maxde=0.;maxnu=0.
        for target,result in zip(truth['rows'],results):
            n,escaped,z,q,ordinary=result;same=n==target['n'] and escaped==target['escaped'];zt=complex(*map(float,target['z']));qt=complex(*map(float,target['q']))
            ze=abs(z-zt);qe=abs(q-qt)/max(abs(qt),1e-300)
            nu=abs(math.log2(math.log(abs(z)))-math.log2(math.log(abs(zt)))) if same and escaped else math.inf
            ok=same and ze<1e-6*max(1,abs(zt)) and qe<1e-6 and nu<1e-6;errors+=not ok;maxde=max(maxde,qe);maxnu=max(maxnu,nu)
            pixels.append(dict(method=name,x=str(target['x']),prefix=init[0],n=n,truth_n=target['n'],correct=ok,state_error=ze,derivative_relative_error=qe,smooth_error=nu,ordinary=ordinary))
        if not order:ref_s=0.
        fields=[name,str(init[0]),repr(ws[init[0]].real),repr(ws[init[0]].imag),repr(center),repr(scale),str(len(init[1])),str(len(init[2]))]
        for poly in [init[1],init[2]]:
            for coeff in poly:fields.extend([repr(coeff.real),repr(coeff.imag)])
        exported.append(' '.join(fields))
        delta=baseline-per
        rows.append(dict(method=name,center=center,radius=scale,order=order,denom=m,tolerance=tol,prefix=init[0],stop=stop,errors=errors,max_derivative_relative_error=maxde,max_smooth_error=maxnu,reference_s=ref_s,setup_s=setup,pixel_s=per,warm_speedup=baseline/per,break_even=math.ceil((setup+ref_s)/delta) if delta>0 else 'never',cold_nine_pixel_speedup=9*baseline/(ref_s+setup+9*per),score=0 if errors else 9*baseline/(ref_s+setup+9*per)))
        print(name,'prefix',init[0],'errors',errors,'warm',round(baseline/per,3),'setup',round(setup,3),flush=True)
    out.mkdir(parents=True,exist_ok=True)
    lines=[str(len(truth['rows']))]
    for target in truth['rows']:
        x=complex(*map(float,target['x']))
        lines.append(' '.join(map(str,[x.real*1e-8,x.imag*1e-8,target['n'],*target['z'],*target['q']])))
    lines.append(str(len(exported)));lines.extend(exported)
    (out/'charts.txt').write_text('\n'.join(lines)+'\n')
    save_csv(out/'local-pixels.csv',pixels);save_csv(out/'local-timings.csv',rows)
    (out/'run.json').write_text(json.dumps(dict(elapsed_s=time.perf_counter()-started,notes='Nine predeclared escaping cusp points, sparse Python; reference/setup charged'),indent=2)+'\n')


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('mode',choices=['freeze','run']);p.add_argument('--truth',type=Path,default=HERE/'local-truth.json');p.add_argument('--out',type=Path,default=HERE/'results/local');p.add_argument('--family',choices=['initial','fair-local','radius','budget'],default='initial');p.add_argument('--dps',type=int,default=100);p.add_argument('--tol',type=float,default=1e-12);a=p.parse_args()
    if a.mode=='freeze':freeze(a.truth,a.dps)
    else:run(a.truth,a.out,a.family,a.tol)
