# PROB-14 probe D: carry dz/dc through the Koenigs jump (for the de and normal columns), in doubles.
# Formula (2) of docs/research/10-9-26/derivative-through-the-koenigs-jump.md:
#   dZ/dc = p' + lam^j K'(q)/K'(w0) * [ D0 - p' - dK/dc(w0) + (4j/lam) w0 K'(w0) ] + dK/dc(q)
# Scored on g = z/(dz/dc) at escape (de = 2|g| ln|z|, normal = arg g): relative error |g/g_truth - 1|.
# usage: python jump_derivative_double.py [pixels_per_width=250]
import sys, random, cmath, math, io, contextlib
from mpmath import mp, mpf, mpc, log
sys.path.insert(0, __import__('os').path.dirname(__file__))
_a=sys.argv; sys.argv=[_a[0],'0']            # import the zone constants without running probe C
with contextlib.redirect_stdout(io.StringIO()):
    import shared_consts_double as S
sys.argv=_a
NPIX=int(sys.argv[1]) if len(sys.argv)>1 else 250
N,R0,n0,Zd,U0,s0,p0d,Cd,lam0d,K0,K1,sgn,hor=S.N,S.R0,S.n0,S.Zd,S.U0,S.s0,S.p0d,S.Cd,S.lam0d,S.K0,S.K1,S.sgn,S.hor
def dhor(k,w): return hor([i*k[i] for i in range(1,len(k))],w)
def pixel(dc,mode):
    d=0j; D=0j
    for n in range(n0):
        zf=Zd[n]+d; D=2*zf*D+1; d=2*Zd[n]*d+d*d+dc
    ds=-4*dc/(2*s0); ds=-4*dc/(2*s0+ds); ds=-4*dc/(2*s0+ds); dp=sgn*ds/2
    pc=p0d+dp; pprime=-1/(2*pc+1)
    u0=U0+d-dp; k=[a+dc*b for a,b in zip(K0,K1)]; lam=lam0d+4*dc; cd=Cd+dc
    w0=u0
    for _ in range(3): w0-=(hor(k,w0)-u0)/dhor(k,w0)
    j=int(math.floor(math.log(R0/abs(w0))/math.log(abs(lam))))
    lj=cmath.exp(j*(cmath.log(lam0d)+cmath.log(1+4*dc/lam0d))); q=w0*lj
    zz=pc+hor(k,q); Kq=dhor(k,q)
    if mode=='full':   dz=pprime+lj*Kq/dhor(k,w0)*(D-pprime-hor(K1,w0)+4*j/lam*w0*dhor(k,w0))+hor(K1,q)
    elif mode=='simple': dz=pprime+lj*Kq*(D-pprime)                    # answer's eq. (8)
    else:              dz=lj*Kq*D                                      # derivative just rides through
    s=0
    while zz.real*zz.real+zz.imag*zz.imag<=1e6 and s<5000:
        dz=2*zz*dz+1; zz=zz*zz+cd; s+=1
    return zz/dz, abs(u0), j
random.seed(13)
print(f'N={N} R0={R0}; error = |g/g_truth - 1| with g = z/(dz/dc) at escape. f32 de needs about 1e-7; the u16 normal needs about 1e-4.')
for wexp in (6,9,12,15,18,24):
    w=mpf(10)**(-wexp); res={'full formula (2)':[], 'simplified (8): keep p\' only':[], 'ride through (drop p\')':[]}; us=[]; js=[]; des=[]
    for _ in range(NPIX):
        off=w*mpc(random.uniform(-.5,.5),random.uniform(-.5,.5)*9/16)
        nu_t,s_t,zT,dzT=S.P.escape(mpc(0),S.C+off,0,mpc(0))
        if nu_t is None: continue
        gt=zT/dzT; de_px=float(2*abs(zT)*log(abs(zT))/abs(dzT)/(w/1920)); des.append(de_px)
        for (name,mode) in zip(res,('full','simple','ride')):
            g,au,j=pixel(complex(off),mode); res[name].append(float(abs(mpc(g)/gt-1)))
        us.append(au); js.append(j)
    print(f'1e-{wexp}: |u0| up to {max(us):.1e}, laps {min(js)}-{max(js)}')
    full=res['full formula (2)']
    worst=sorted(zip(full,des),reverse=True)[:3]
    print('    worst full-formula pixels (rel err, de in px): '+', '.join(f'{e:.0e} @ de {d:.1e}' for e,d in worst))
    for name,e in res.items():
        ok=[x for x,d in zip(e,des) if d>=0.01]; ok.sort(); e2=sorted(e)
        print(f'    {name:30s} p50 {e2[len(e2)//2]:.1e}  max {e2[-1]:.1e}  over 1e-4: {sum(x>1e-4 for x in e2)}/{len(e2)}  | pixels with de >= 0.01 px: max {ok[-1]:.1e} ({len(ok)} px)',flush=True)
