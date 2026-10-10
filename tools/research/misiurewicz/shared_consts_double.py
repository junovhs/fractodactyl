# PROB-14 probe C: the mid-band pixel in IEEE doubles with ONE set of jump constants per zone.
# Constants at C (k_n, dk_n/dc, cycle point, reference orbit) are built once at high precision;
# per pixel only dc = c - C is used: perturbation approach, closed-form cycle shift, exact lambda(c),
# K with first-order coefficient correction, plain double finish.
# usage: python shared_consts_double.py [pixels_per_width=250] [N=18] [R0=0.03]
import sys, random, cmath, math
from mpmath import mp, mpf, mpc, sqrt, log
sys.path.insert(0, __import__('os').path.dirname(__file__))
mp.dps = 70
import own_c_koenigs as P   # reuses C, cyc, kcoef, escape (runs its own report on import)
C=P.C; NPIX=int(sys.argv[1]) if len(sys.argv)>1 else 250
N=int(sys.argv[2]) if len(sys.argv)>2 else 18; R0=float(sys.argv[3]) if len(sys.argv)>3 else 0.03
# ---- per-zone constants (high precision once, stored as doubles) ----
z=mpc(0); orb=[]
for n in range(40): z=z*z+C; orb.append(z)
pA,pB=P.cyc(C)
best=min(range(20,40),key=lambda i:min(abs(orb[i]-pA),abs(orb[i]-pB))); n0=best+1
p0,q0=(pA,pB) if abs(orb[best]-pA)<abs(orb[best]-pB) else (pB,pA)
sgn=1 if p0==pA else -1                       # p = (-1 + sgn*s)/2
lam0,k0=P.kcoef(C,p0,q0,N)
h=mpf(10)**-25
def kat(c):
    a,b=P.cyc(c); pp,qq=(a,b) if sgn==1 else (b,a); return P.kcoef(c,pp,qq,N)[1]
kp,km=kat(C+h),kat(C-h); k1=[(x-y)/(2*h) for x,y in zip(kp,km)]; k2=[(x-2*m_+y)/(h*h) for x,m_,y in zip(kp,k0,km)]
Zd=[complex(0)]+[complex(x) for x in orb[:n0]]          # Z_0..Z_n0 of the reference
U0=complex(orb[best]-p0); s0=complex(sqrt(-3-4*C)); p0d=complex(p0); Cd=complex(C); lam0d=complex(lam0)
K0=[complex(x) for x in k0]; K1=[complex(x) for x in k1]
print(f'\nzone constants: n0={n0} |U0|={abs(U0):.2e} lambda0={lam0d:.5f} N={N} R0={R0}')
# check of the pasted Taylor-error numbers: sup over |w|=R0 of |K_c - (k + dc k')| at |dc|=r
for r in ('1e-9','1e-12'):
    dc=mpf(r)*mpc(0.6,0.8); kc=kat(C+dc)
    e1=max(abs(sum((kc[n]-k0[n]-dc*k1[n])*(R0*mp.expj(t))**n for n in range(2,N+1))) for t in [0.1*i for i in range(63)])
    e0=max(abs(sum((kc[n]-k0[n])*(R0*mp.expj(t))**n for n in range(2,N+1))) for t in [0.1*i for i in range(63)])
    print(f'  |dc|={r}: coefficient error in K at |w|=R0: frozen k {float(e0):.1e}, first-order {float(e1):.1e}')
def hor(k,w):
    r=0j
    for x in reversed(k): r=r*w+x
    return r
def pixel(dc,first_order,exact_lam=True):
    # approach by perturbation against the reference
    d=0j
    for n in range(n0): d=2*Zd[n]*d+d*d+dc
    ds=-4*dc/(2*s0); ds=-4*dc/(2*s0+ds); ds=-4*dc/(2*s0+ds)      # sqrt(-3-4c)-sqrt(-3-4C)
    dp=sgn*ds/2
    u0=U0+d-dp
    k=[a+dc*b for a,b in zip(K0,K1)] if first_order else K0
    lam=lam0d+4*dc if exact_lam else lam0d
    cd=Cd+dc; pc=p0d+dp
    if abs(u0)>=R0/abs(lam):
        zz=Zd[n0]+d; n=n0; j=0
    else:
        w0=u0
        for _ in range(3):
            w0-= (hor(k,w0)-u0)/hor([i*k[i] for i in range(1,N+1)],w0)
        j=int(math.floor(math.log(R0/abs(w0))/math.log(abs(lam))))
        w1=w0*cmath.exp(j*(cmath.log(lam0d)+ (cmath.log(1+4*dc/lam0d) if exact_lam else 0)))  # keeps dc below 1e-16
        zz=pc+hor(k,w1); n=n0+2*j
    s=0
    while zz.real*zz.real+zz.imag*zz.imag<=1e6 and s<5000:
        zz=zz*zz+cd; s+=1
    if s>=5000: return None,s
    return n+s+1-math.log2(math.log2(abs(zz))), n0+s
random.seed(11)
print('error = |dnu|*ln2*de_px, 1920 px frame, bar 1e-3 px; every arm is IEEE double per pixel')
for wexp in (6,9,12,15,18,24):
    w=mpf(10)**(-wexp); arms={'first-order k, exact lambda(c)':(True,True),'frozen k, exact lambda(c)':(False,True),'frozen k, frozen lambda':(False,False)}
    res={a:[] for a in arms}; left=[]
    for _ in range(NPIX):
        off=w*mpc(random.uniform(-.5,.5),random.uniform(-.5,.5)*9/16); c=C+off
        nu_t,s_t,zT,dzT=P.escape(mpc(0),c,0,mpc(0))
        if nu_t is None: continue
        de_px=float(2*abs(zT)*log(abs(zT))/abs(dzT)/(w/1920))
        for a,(fo,el) in arms.items():
            nu,st=pixel(complex(off),fo,el)
            res[a].append(float('inf') if nu is None else abs(nu-float(nu_t))*math.log(2)*de_px)
            if a.startswith('first'): left.append(st)
    left.sort(); print(f'1e-{wexp}: steps left {left[len(left)//2]}')
    for a,e in res.items():
        e.sort(); print(f'    {a:32s} max {e[-1]:.1e}  p50 {e[len(e)//2]:.1e}  bad {sum(x>1e-3 for x in e)}/{len(e)}',flush=True)
