# PROB-12/14 probe E: Koenigs jump at a LONG repelling cycle found on a real published zoom
# (Maths Town "Eye of the Universe" centre; r = 197 and r = 655 dwells from orbit_laps.py).
# Builds the linearizer with the general-period recurrences of
# docs/research/10-9-26/general-period-cycle-acceleration.md and asks: how many of the observed laps
# can an N-term jump cover, and how accurate is the jumped point against plain iteration?
# usage: python general_cycle_jump.py [NSTEPS=30000] [periods=197,655]
import sys, re, os, time
import numpy as np, mpmath as mp
here=os.path.dirname(os.path.abspath(__file__))
src=open(os.path.join(here,'orbit_laps.py')).read()
RE=''.join(re.findall(r"'([-0-9.]+)'", src[src.index('RE = ('):src.index('IM = (')]))
IM=''.join(re.findall(r"'([-0-9.]+)'", src[src.index('IM = ('):src.index('def main')]))
NST=int(sys.argv[1]) if len(sys.argv)>1 else 30000
PER=[int(x) for x in (sys.argv[2] if len(sys.argv)>2 else '197,655').split(',')]
mp.mp.dps=max(len(RE),len(IM))+50
t0=time.time(); c=mp.mpc(RE,IM); z=mp.mpc(0); zd=np.empty(NST+1,complex); zd[0]=0; keep={}
WORK=90
for n in range(1,NST+1):
    z=z*z+c; zd[n]=complex(z); keep[n]=z
print(f'centre orbit: {NST} steps at {mp.mp.dps} digits, {time.time()-t0:.1f} s')
def series_mul(x,y,N):
    r=[mp.mpc(0)]*(N+1)
    for i in range(1,N+1):
        if x[i]==0: continue
        for j in range(1,N+1-i): r[i+j]+=x[i]*y[j]
    return r
def build(p0,cc,r,N):
    pts=[p0]
    for i in range(r-1): pts.append(pts[-1]**2+cc)
    U=[mp.mpc(0)]*(N+1); U[1]=mp.mpc(1)                      # U_{i+1} = 2 p_i U_i + U_i^2
    for i in range(r):
        sq=series_mul(U,U,N); U=[2*pts[i]*U[m]+sq[m] for m in range(N+1)]
    a=U; lam=a[1]
    k=[mp.mpc(0)]*(N+1); k[1]=mp.mpc(1)
    for n in range(2,N+1):                                   # (lam^n - lam) k_n = sum_m a_m [w^n] K^m
        P=mp.mpc(0); pw=k[:]                                  # pw = K^1 truncated at degree n
        kk=k[:n]+[mp.mpc(0)]*(N+1-n)                          # uses k_1..k_{n-1} only
        pw=kk[:]
        for m in range(2,n+1):
            pw=series_mul(pw,kk,n) if len(pw)>n else pw
            P+=a[m]*pw[n]
        k[n]=P/(lam**n-lam)
    return pts,lam,a,k
def horner(k,w):
    s=mp.mpc(0)
    for x in reversed(k): s=s*w+x
    return s
for r in PER:
    close=np.abs(zd[r:]-zd[:-r])<1e-3
    d=np.diff(np.concatenate([[0],close.astype(np.int8),[0]])); s,e=np.nonzero(d==1)[0],np.nonzero(d==-1)[0]
    if len(s)==0: print(f'r={r}: no dwell in the first {NST} steps'); continue
    i=int(np.argmax(e-s)); a0,b0=int(s[i]),int(e[i]); laps=(b0-a0)/r
    mp.mp.dps=WORK
    cc=mp.mpc(c.real,c.imag); n0=max(a0,1)
    # Newton for the cycle point near z_{n0}, F = f^r
    p=mp.mpc(keep[n0].real,keep[n0].imag)
    for _ in range(60):
        x=p; A=mp.mpc(1)
        for _i in range(r): A=2*x*A; x=x*x+cc
        step=(x-p)/(A-1); p-=step
        if abs(step)<mp.mpf(10)**(-WORK+8): break
    lamabs=abs(A)
    print(f'\nr={r}: dwell steps {a0}..{b0} = {laps:.1f} laps; |lambda|={float(lamabs):.3f}; entry offset |u0|={float(abs(keep[n0]-p)):.2e}')
    for N in (8,16,32):
        t1=time.time(); pts,lam,a,k=build(p,cc,r,N); tb=time.time()-t1
        u0=mp.mpc(keep[n0].real,keep[n0].imag)-p
        w0=u0
        for _ in range(8):
            dK=horner([i*k[i] for i in range(1,N+1)],w0); w0-=(horner(k,w0)-u0)/dK
        rows=[]; jgood=0
        for j in range(1,int(laps)+4):
            n=n0+r*j
            if n>NST: break
            true=mp.mpc(keep[n].real,keep[n].imag)-p
            pred=horner(k,w0*lam**j)
            rel=abs(pred-true)/abs(true)
            rows.append((j,float(abs(true)),float(rel)))
            if rel<1e-10: jgood=j
        amax=max(float(abs(a[m]))**(1/(m-1)) for m in range(2,N+1) if a[m]!=0)
        kg=max(float(abs(k[m]))**(1/(m-1)) for m in range(2,N+1) if k[m]!=0)
        ug=[x for x in rows if x[0]==jgood]
        print(f'  N={N:2d}: build {tb:.1f} s; |a_m|^(1/(m-1)) up to {amax:.2e}, |k_n|^(1/(n-1)) up to {kg:.2e};'
              f' jump accurate (rel 1e-10) for {jgood} laps = {jgood*r} steps, out to |u|={ug[0][1] if ug else 0:.2e}')
        if N==32: print('        lap: |u| true, rel err  ->  '+'  '.join(f'{j}: {u:.1e}, {e:.0e}' for j,u,e in rows[-9:]))
    mp.mp.dps=max(len(RE),len(IM))+50
