# PROB-14 probe F: which stages of the mid-band pixel survive IEEE binary32 (GPU-native floats)?
# Same zone constants and pipeline as shared_consts_double.py; each stage's arithmetic is forced to
# float32/complex64 or left in float64/complex128. Error = |dnu|*ln2*de_px against 70-digit iteration.
# usage: python float32_stages.py [pixels_per_width=250]
import sys, random, math, io, contextlib, warnings
import numpy as np
from mpmath import mp, mpf, mpc, log
sys.path.insert(0, __import__('os').path.dirname(__file__))
_a=sys.argv; sys.argv=[_a[0],'0']
with contextlib.redirect_stdout(io.StringIO()):
    import shared_consts_double as S
sys.argv=_a; warnings.simplefilter('ignore')
NPIX=int(sys.argv[1]) if len(sys.argv)>1 else 250
N,R0,n0,sgn=S.N,S.R0,S.n0,S.sgn
C64,C128,F32,F64=np.complex64,np.complex128,np.float32,np.float64
JMAX=420
TJ=[S.lam0**j for j in range(JMAX)]                       # lambda0^j at 70 digits, rounded once per precision
def consts(ct):
    return dict(Z=[ct(z) for z in S.Zd], U0=ct(S.U0), s0=ct(S.s0), p0=ct(S.p0d), C=ct(S.Cd), lam0=ct(S.lam0d),
                K0=[ct(x) for x in S.K0], K1=[ct(x) for x in S.K1], T=[ct(complex(t)) for t in TJ])
K={C64:consts(C64),C128:consts(C128)}
def hor(k,w,ct):
    r=ct(0)
    for x in reversed(k): r=r*w+x
    return r
def pixel(dc,p):
    """p: dict stage -> complex type; p['jump'] in {'exp','table'}"""
    ct=p[1]; k=K[ct]; d=ct(0); dcc=ct(dc)
    for n in range(n0): d=ct(2)*k['Z'][n]*d+d*d+dcc
    ds=ct(-4)*dcc/(ct(2)*k['s0']); ds=ct(-4)*dcc/(ct(2)*k['s0']+ds); dp=ct(sgn)*ds/ct(2)
    ct3=p[3]; u0=ct3(K[ct3]['U0'])+ct3(d)-ct3(dp)
    ct4=p[4]; k4=K[ct4]; kk=[a+ct4(dc)*b for a,b in zip(k4['K0'],k4['K1'])]; u=ct4(u0); w0=u
    for _ in range(3):
        w0=w0-(hor(kk,w0,ct4)-u)/hor([ct4(i)*kk[i] for i in range(1,N+1)],w0,ct4)
    ct5=p[5]; k5=K[ct5]; lam=k5['lam0']+ct5(4*dc)
    j=int(math.floor(math.log(R0/abs(complex(w0)))/math.log(abs(complex(lam)))))
    if p['jump']=='exp':
        w1=ct5(w0)*np.exp(ct5(j)*np.log(lam))
    else:
        w1=ct5(w0)*k5['T'][j]*(ct5(1)+ct5(j)*ct5(4*dc)/k5['lam0'])
    ct6=p[6]; k6=K[ct6]; kk6=[a+ct6(dc)*b for a,b in zip(k6['K0'],k6['K1'])]
    Kw=hor(kk6,ct6(w1),ct6)
    cta=p['add']; pc=cta(K[C128]['p0']+C128(complex(dp)))          # p(c) known to the precision of the add
    zz=pc+cta(Kw)
    ct7=p[7]; cd=ct7(K[C128]['C']+C128(dc)); zz=ct7(zz); s=0
    while zz.real*zz.real+zz.imag*zz.imag<=1e6 and s<5000:
        zz=zz*zz+cd; s+=1
    if s>=5000: return None
    return n0+2*j+s+1-math.log2(math.log2(abs(complex(zz))))
D=C128; F=C64
def arm(**kw):
    p={1:D,3:D,4:D,5:D,6:D,7:D,'add':D,'jump':'exp'}; p.update(kw); return p
ARMS=[('all double (baseline)',arm()),
      ('stages 1-4 float',arm(**{'1':0})|{1:F,3:F,4:F}),
      ('jump: float exp(j*log lambda)',arm()|{5:F}),
      ('jump: float table lambda^j',arm()|{5:F,'jump':'table'}),
      ('landing: float Horner, double add',arm()|{6:F}),
      ('landing: float Horner + float add',arm()|{6:F,'add':F}),
      ('finish in float',arm()|{7:F}),
      ('MIX: float 1-6 (table), double add + finish',{1:F,3:F,4:F,5:F,6:F,7:D,'add':D,'jump':'table'}),
      ('ALL float (table jump)',{1:F,3:F,4:F,5:F,6:F,7:F,'add':F,'jump':'table'})]
for a in ARMS: a[1].pop('1',None)
random.seed(17)
print(f'{NPIX} pixels per width; error in px of a 1920-px frame; columns: max, median, count over 1e-3, count over 1e-2')
for wexp in (9,15,24):
    w=mpf(10)**(-wexp); res={n:[] for n,_ in ARMS}
    for _ in range(NPIX):
        off=w*mpc(random.uniform(-.5,.5),random.uniform(-.5,.5)*9/16)
        nu_t,s_t,zT,dzT=S.P.escape(mpc(0),S.C+off,0,mpc(0))
        if nu_t is None: continue
        de_px=float(2*abs(zT)*log(abs(zT))/abs(dzT)/(w/1920))
        for n,p in ARMS:
            nu=pixel(complex(off),p); res[n].append(float('inf') if nu is None else abs(nu-float(nu_t))*math.log(2)*de_px)
    print(f'width 1e-{wexp}')
    for n,e in res.items():
        e.sort(); print(f'   {n:44s} max {e[-1]:.1e}  p50 {e[len(e)//2]:.1e}  >1e-3: {sum(x>1e-3 for x in e):3d}  >1e-2: {sum(x>1e-2 for x in e):3d}  of {len(e)}',flush=True)
