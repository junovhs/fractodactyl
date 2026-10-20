#!/usr/bin/env python3
"""Addendum: frame-shared fourth-order parameter-series initializer (SA/AT-like).
Separate coefficient construction from per-pixel polynomial evaluation and fallback.
Reference uses mpmath 100 dps. Run: python fractal_la_cached_series.py
"""
from pathlib import Path
import csv,math,time,statistics
import mpmath as mp
from fractal_la_experiment import CASES,WIDTHS,OFFSETS,N,hp_reference,hp_pixel,EscapeGuard,smooth


def series_build(ref,rho,order=4,tol=1e-6):
    coeff=[0j]*(order+1);error=0.; last=0;best=coeff[:];best_err=0.
    for j in range(len(ref)-1):
        v=[0j]*(order+1)
        v[1]=2*ref[j]*coeff[1]+1
        for k in range(2,order+1):
            v[k]=2*ref[j]*coeff[k]+sum(coeff[h]*coeff[k-h] for h in range(1,k))
        tail=sum(abs(coeff[a]*coeff[b])*rho**(a+b) for a in range(1,order+1) for b in range(1,order+1) if a+b>order)
        pbound=sum(abs(coeff[k])*rho**k for k in range(1,order+1))
        error=2*(abs(ref[j])+pbound)*error+error*error+tail
        coeff=v
        pb=sum(abs(coeff[k])*rho**k for k in range(1,order+1))
        if error>rho*tol or abs(ref[j+1])+pb+error>=2: break
        last=j+1;best=coeff[:];best_err=error
    return last,best,best_err


def evaluate(ref,hpr,dc,init):
    end,coeff,cert=init
    u=sum(coeff[k]*dc**k for k in range(1,len(coeff)))
    d=sum(k*coeff[k]*dc**(k-1) for k in range(1,len(coeff)))
    n=end;ordinary=0;guard=EscapeGuard(hpr,ref)
    while n<N and not guard.escaped(n,u):
        d=2*(ref[n]+u)*d+1
        u=u*(2*ref[n]+u)+dc
        n+=1;ordinary+=1
    return u,d,n,ordinary


def main():
    mp.mp.dps=100;rows=[];preps=[]
    for name,(re,im) in CASES.items():
        c=mp.mpc(mp.mpf(re),mp.mpf(im));hp=hp_reference(c,N);ref=[complex(z) for z in hp]
        for w in WIDTHS:
            rho=10.**(-w)
            t=time.perf_counter();init=series_build(ref,rho);tprep=time.perf_counter()-t
            preps.append((name,w,tprep,init[0]))
            for ox,oy in OFFSETS:
                dchp=mp.mpc(str(ox),str(oy))*mp.power(10,-w);dc=complex(dchp)
                hu,hd,hn=hp_pixel(hp,dchp,N)
                u,d,n,ordinary=evaluate(ref,hp,dc,init)
                rows.append(dict(case=name,width=w,offset=f'{ox:+g}{oy:+g}i',init_length=init[0],
                    ordinary=ordinary,escape_ref=hn,escape_at=n,correct=int(hn==n),
                    state_error=abs((ref[n]+u)-(complex(hp[n])+hu)) if n==hn else float('nan'),
                    deriv_rel=abs(d-hd)/max(1,abs(hd)) if n==hn else float('nan'),
                    smooth_error=abs(smooth(ref,n,u,hp)-smooth(ref,n,hu,hp)) if n==hn and n<N else float('nan'),
                    preprocessing_s=tprep))
        print(name,'finished',flush=True)
    with Path('/mnt/data/fractal_la_cached_series.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    print('Cached series accuracy',sum(r['correct'] for r in rows),len(rows),'ordinary',statistics.mean(r['ordinary'] for r in rows),
        'worst_der_rel',max(r['deriv_rel'] for r in rows if math.isfinite(r['deriv_rel'])),
        'worst_smooth',max((r['smooth_error'] for r in rows if math.isfinite(r['smooth_error'])),default=float('nan')))
    print('Preprocessing min/mean/max seconds',min(t[2] for t in preps),statistics.mean(t[2] for t in preps),max(t[2] for t in preps))
    for name in ['attracting_-1','seahorse_edge','misiurewicz_i']:
        c=CASES[name];hp=hp_reference(mp.mpc(mp.mpf(c[0]),mp.mpf(c[1])),N);ref=[complex(z) for z in hp];rho=1e-20
        init=series_build(ref,rho); dcs=[complex(ox,oy)*rho for ox,oy in OFFSETS]
        times=[]
        for _ in range(3):
            t=time.perf_counter()
            for i in range(300):
                for dc in dcs:evaluate(ref,hp,dc,init)
            times.append((time.perf_counter()-t)/900)
        print('cached-series4',name,round(1e6*statistics.median(times),2),'microseconds/pixel','init',init[0])

if __name__=='__main__':main()
