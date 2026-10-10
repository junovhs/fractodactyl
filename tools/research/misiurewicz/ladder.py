# Probe G: the Misiurewicz ladder. Minibrot centres c_n -> m = M(24,2) with c_n - m ~ A rho^-n and
# size ~ |B| |rho|^-2n (docs/research/10-9-26/misiurewicz-ladders.md).
#  1. check the answer's L = 1 ladder (period 25 + 2n) and its two-term predictor at n = 100, 200, 366;
#  2. put the v0 nucleus (period 764) on its own ladder and predict + Newton-refine rungs near sizes
#     1e-100 and 1e-1000;  3. count close returns per pixel at the 1e-100 rung (is cost flat?).
# usage: python ladder.py
import sys, time, random
from mpmath import mp, mpf, mpc, log, log10, floor, fabs, nstr
RE='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
IM='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
def nucleus(c,P,iters=40,tol=None):
    """Newton on f^P_c(0) = 0; returns (c, |last step|, size estimate 1/(b*dz/dc))"""
    for it in range(iters):
        z=mpc(0); d=mpc(0)
        for _ in range(P): d=2*z*d+1; z=z*z+c
        step=z/d; c=c-step
        if abs(step)<tol: break
    z=mpc(0); d=mpc(0); b=mpc(1)
    for j in range(P):
        d=2*z*d+1; z=z*z+c
        if j<P-1: b*=2*z
    return c,abs(step),1/(b*d),it+1
def misiurewicz(c):
    for _ in range(60):
        z=mpc(0); d=mpc(0); zs=[]; ds=[]
        for k in range(26): d=2*z*d+1; z=z*z+c; zs.append(z); ds.append(d)
        g=zs[25]-zs[23]; dg=ds[25]-ds[23]; c-=g/dg
        if abs(g/dg)<mpf(10)**(-mp.dps+10): break
    return c
t0=time.time()
mp.dps=1150
C=mpc(mpf(RE),mpf(IM)); m=misiurewicz(C); rho=4*(m+1); L=abs(rho)
print(f'm - C(v0) = {nstr(m-C,6)}  |m - C| = {nstr(abs(m-C),4)};  rho = {nstr(rho,12)}  |rho| = {nstr(L,10)}')
# ---- 1. the answer's L = 1 ladder ----
A=mpc('0.00146873010307155719','-0.00176557355983146933'); B=mpc('0.00002535504256552955','0.00002789044719997487')
Cc=mpc('0.00099628274546597131','-0.00115707322224533577')
print('\n1. answer\'s ladder, period 25 + 2n: predicted centre vs Newton solution (errors in units of S_n = |B||rho|^-2n)')
for n in (100,200,366):
    mp.dps=int(2.2*n*float(log10(L)))+60
    mm=mpc(m.real,m.imag); r=4*(mm+1); lam=r**(-n); Sn=abs(B)*abs(r)**(-2*n)
    p1=mm+A*lam; p2=p1+(Cc-n*(4/r)*A*A)*lam*lam
    c,st,size,its=nucleus(p2,25+2*n,tol=Sn*mpf(10)**-25)
    print(f'   n={n:3d} P={25+2*n}: Newton {its} its; first-order error {nstr(abs(c-p1)/Sn,4)} S_n; two-term error {nstr(abs(c-p2)/Sn,3)} S_n;'
          f' (c_n-m)rho^n/A = {nstr((c-mm)*r**n/A,8)}; size/(B rho^-2n) = {nstr(size/(B*lam*lam),8)}')
# ---- 2. the v0 ladder ----
mp.dps=1150
print('\n2. v0 ladder (period 764 + 2k). Rung k=0 is the v0 nucleus itself.')
c0,st,size0,its=nucleus(C,764,tol=mpf(10)**-140)
print(f'   k=0: |c0 - C(v0, 100 digits)| = {nstr(abs(c0-C),3)}; |c0 - m| = {nstr(abs(c0-m),5)}; size = {nstr(abs(size0),5)}, arg {nstr(mp.arg(size0),4)}')
rungs={}
for k in (1,2,409,7676):
    P=764+2*k; lam=rho**(-k)
    g1=m+(c0-m)*lam
    if 1 in rungs and 2 in rungs:      # second-order from the two neighbours: a_k=(c_k-m)rho^k = a + (e - k f) rho^-k
        a1=(rungs[1]-m)*rho; a2=(rungs[2]-m)*rho**2; a0=(c0-m)
        # three unknowns a,e,f from k=0,1,2:  a0=a+e ; a1=a+(e-f)/rho ; a2=a+(e-2f)/rho^2
        x1=1/rho; x2=1/rho**2
        # solve linear system
        import mpmath as M
        S=M.lu_solve(M.matrix([[1,1,0],[1,x1,-x1],[1,x2,-2*x2]]),M.matrix([a0,a1,a2]))
        a,e,f=S[0],S[1],S[2]
        g2=m+(a+(e-k*f)*lam)*lam
    else: g2=g1
    sz=abs(size0)*L**(-2*k)
    t1=time.time(); c,st,size,its=nucleus(g2,P,tol=sz*mpf(10)**-20); rungs[k]=c
    print(f'   k={k:4d} P={P:5d}: Newton {its} its ({time.time()-t1:.1f} s), last step {nstr(st/sz,2)} sizes; guess error: first-order {nstr(abs(c-g1)/sz,4)}, fitted two-term {nstr(abs(c-g2)/sz,3)} sizes;'
          f' |c-m| = {nstr(abs(c-m),4)}; size = {nstr(abs(size),4)} (ratio to |rho|^-2k law {nstr(abs(size)/sz,6)})')
open('rungs.txt','w').write('\n'.join(f'{k} {764+2*k} {mp.nstr(c.real,1120)} {mp.nstr(c.imag,1120)}' for k,c in rungs.items()))
# ---- 3. returns per pixel at the deep rung vs v0 ----
print('\n3. close returns per pixel (|z| < 1e3 * frame width^(1/2)... counted as steps where |z_n| is a new minimum-scale return of the nucleus period)')
random.seed(5)
for k,cn,size in ((0,c0,abs(size0)),(409,rungs[409],abs(size0)*L**(-818))):
    P=764+2*k; mp.dps=int(-float(log10(size)))+40
    for wmul in (5,50,5000):
        ks=[]; steps=[]
        for _ in range(12):
            c=mpc(cn.real,cn.imag)+size*wmul*mpc(random.uniform(-.5,.5),random.uniform(-.5,.5))
            z=mpc(0); n=0; ret=0
            while abs(z)<1e3 and n<40*P:
                z=z*z+c; n+=1
                if n%P==0 and abs(z)<mpf(size)**mpf('0.25'): ret+=1
            ks.append(ret); steps.append(n)
        print(f'   rung k={k:3d} (P={P}, size {nstr(size,3)}), frame {wmul:5d} sizes wide: returns per pixel {min(ks)}-{max(ks)}, direct steps {min(steps)}-{max(steps)}')
print(f'total {time.time()-t0:.0f} s')
