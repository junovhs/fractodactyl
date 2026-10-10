# PROB-14 probe A/B: Koenigs jump over the 2-cycle spiral at each pixel's OWN parameter c.
# Also checks the explicit radius theorem (deep-research answer 2026-10-09) on the v0 numbers.
# usage: python own_c_koenigs.py [pixels_per_width=120] [N=24] [R0=0.03]
import sys, random
from mpmath import mp, mpf, mpc, sqrt, log, floor, fabs
mp.dps = 70
RE='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
IM='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
C=mpc(mpf(RE),mpf(IM))
NPIX=int(sys.argv[1]) if len(sys.argv)>1 else 120
N=int(sys.argv[2]) if len(sys.argv)>2 else 24
R0=mpf(sys.argv[3]) if len(sys.argv)>3 else mpf('0.03')
ESC=mpf(10)**6; MAXIT=20000; PXW=1920

def cyc(c):
    s=sqrt(-3-4*c); return (-1+s)/2, (-1-s)/2
def kcoef(c,p,q,n):
    """inverse linearizer K(w)=w+sum k_j w^j of G(u)=F_c(p+u)-p, F=f^2: (lam^j-lam)k_j=[w^j](aK^2+bK^3+K^4)"""
    lam=4*(c+1); a=4*p*p+2*q; b=4*p
    k=[mpc(0),mpc(1)]+[mpc(0)]*(n-1)
    def mul(x,y,upto):
        r=[mpc(0)]*(upto+1)
        for i,xi in enumerate(x[:upto+1]):
            if xi==0: continue
            for j,yj in enumerate(y[:upto+1-i]): r[i+j]+=xi*yj
        return r
    for j in range(2,n+1):
        K2=mul(k,k,j); K3=mul(K2,k,j); K4=mul(K2,K2,j)
        k[j]=(a*K2[j]+b*K3[j]+K4[j])/(lam**j-lam)
    return lam,k
def horner(k,w):
    r=mpc(0)
    for x in reversed(k): r=r*w+x
    return r
def escape(z,c,n,dz=None):
    """plain iteration from step n; returns (nu, steps_used, z, dz)"""
    s=0
    while abs(z)**2<=ESC and n<MAXIT:
        if dz is not None: dz=2*z*dz+1
        z=z*z+c; n+=1; s+=1
    if n>=MAXIT: return None,s,z,dz
    return n+1-log(log(abs(z),2),2), s, z, dz

def _report():
    p0,q0=cyc(C); L=abs(4*(C+1)); rho_c=(L-1)/8; d0=-3-4*C; m=rho_c*sqrt(2/abs(d0)); qq=(L+1)/2
    print(f'v0: lambda={complex(4*(C+1)):.5f} |lambda|={float(L):.5f} rho_c={float(rho_c):.4f}')
    for nm,(pp,qo) in (('p+',(p0,q0)),('p-',(q0,p0))):
        A=4*(abs(pp)+m)**2+2*(abs(qo)+m); B=4*(abs(pp)+m); D=qq*(qq-1); R=min(mpf(1)/4,D/(16*(A+B+1)))
        lam,k=kcoef(C,pp,qo,60)
        maj=sum(abs(k[j])*R**j for j in range(2,61))
        # empirical radius: |k_n|^(1/n) growth
        growth=max(abs(k[j])**(mpf(1)/j) for j in range(30,61))
        def err(w,n): return abs(sum(k[j]*w**j for j in range(n+1,61)))
        print(f' {nm}: R={float(R):.3e} certified rho_w<=R/10={float(R/10):.3e}; sum|k_n|R^n={float(maj):.3e} (claim <= R/4={float(R/4):.3e});'
              f' |k_n|^(1/n)~{float(growth):.2f} => series radius ~{float(1/growth):.3f};'
              f' trunc err at |w|=0.03: N=18 {float(err(R0,18)):.1e}, N=24 {float(err(R0,24)):.1e}; theorem bound at |w|=R/10,N=18: {float(R/20*mpf(0.5)**19):.1e}')
        extra=log(R0/(R/10))/log(L)
        print(f'     staying inside the certified radius instead of 0.03 costs {float(extra):.0f} more laps = {float(2*extra):.0f} plain steps per pixel')

    # ---- probe ----
    random.seed(7)
    print(f'\npixels/width {NPIX}, N={N}, R0={float(R0)}; error = |dnu|*ln2*de_px (1920 px frame); pass bar 1e-3 px')
    print('width | truth steps med | variant: max px err, pixels >1e-3, median steps left (of which finish)')
    for wexp in (6,9,12,15,18,24):
        w=mpf(10)**(-wexp); rows={'own-c jump + own-c finish':[], 'own-c jump + C finish':[], 'C jump + C finish (control)':[]}
        tsteps=[]
        for _ in range(NPIX):
            c=C+w*mpc(random.uniform(-.5,.5),random.uniform(-.5,.5)*9/16)
            nu_t,s_t,zT,dzT=escape(mpc(0),c,0,mpc(0))
            if nu_t is None: continue
            de_px=2*abs(zT)*log(abs(zT))/abs(dzT)/(w/PXW); tsteps.append(s_t)
            # approach: exact steps to the first close pass of the 2-cycle
            z=mpc(0); orb=[]
            for n in range(1,41):
                z=z*z+c; orb.append(z)
            for name,(cj,cf) in (('own-c jump + own-c finish',(c,c)),('own-c jump + C finish',(c,C)),('C jump + C finish (control)',(C,C))):
                pp,qo=cyc(cj)
                best=min(range(20,40),key=lambda i:min(abs(orb[i]-pp),abs(orb[i]-qo)))
                n0=best+1; z0=orb[best]
                if abs(z0-qo)<abs(z0-pp): pp,qo=qo,pp
                u0=z0-pp
                lam,k=kcoef(cj,pp,qo,N)
                if abs(u0)>=R0/abs(lam):
                    nu,s,_,_=escape(z0,cf,n0); used=n0+s; fin=s
                else:
                    j=int(floor(log(R0/abs(u0))/log(abs(lam))))
                    # H(u0)=u0+O(u0^2): refine w0 by Newton on K(w0)=u0 (u0 is tiny in the mid band)
                    w0=u0
                    for _i in range(4):
                        Kw=horner(k,w0); dK=horner([i*k[i] for i in range(1,N+1)],w0); w0-=(Kw-u0)/dK
                    z1=pp+horner(k,w0*lam**j)
                    nu,s,_,_=escape(z1,cf,n0+2*j); used=n0+s; fin=s
                if nu is None: rows[name].append((mpf('inf'),used,fin)); continue
                rows[name].append((abs(nu-nu_t)*log(2)*de_px,used,fin))
        tsteps.sort()
        print(f'1e-{wexp} | {tsteps[len(tsteps)//2]}')
        for name,r in rows.items():
            errs=sorted(x[0] for x in r); us=sorted(x[1] for x in r); fs=sorted(x[2] for x in r)
            print(f'   {name:28s}: max {float(errs[-1]):.1e}  p50 {float(errs[len(errs)//2]):.1e}  bad {sum(e>1e-3 for e in errs)}/{len(errs)}  steps left {us[len(us)//2]} (finish {fs[len(fs)//2]})',flush=True)

if __name__=='__main__': _report()
