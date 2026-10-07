import mpmath as mp, math, sys, random
from multiprocessing import Pool
P=764; DPS=110; MAXIT=400000; R2=mp.mpf(10)**20
mp.mp.dps=DPS
C=mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502','0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')
def setup():
    mp.mp.dps=DPS
    z=mp.mpc(0); dz=mp.mpc(0)
    for i in range(P):
        dz=2*z*dz+1; z=z*z+C
    return z, dz   # z_p(C) (~0), B = dz_p/dc
def nu(c):
    """smooth escape count and |dz/dc|-based distance (complex-plane units)"""
    mp.mp.dps=DPS
    z=mp.mpc(0); dz=mp.mpc(0)
    for n in range(MAXIT):
        dz=2*z*dz+1; z=z*z+c
        a=z.real*z.real+z.imag*z.imag
        if a>R2:
            az=mp.sqrt(a)
            v=n+1-mp.log(mp.log(az))/mp.log(2)
            de=2*az*mp.log(az)/abs(dz)
            return float(v), de, n+1
    return None, None, MAXIT
def work(args):
    mp.mp.dps=DPS
    d, B, w = args
    a=nu(C+d); 
    exact_cp = None
    # closed form shallow parameter
    cp=C+d+B*B*d*d
    b=nu(cp)
    if a[0] is None or b[0] is None: return (a[0] is None, b[0] is None, None, None, n_or(a), n_or(b))
    dnu=a[0]-(b[0]+P)
    px=w/480
    disp=abs(dnu)*math.log(2)*float(a[1]/px)
    return (False, False, dnu, disp, a[2], b[2])
def n_or(x): return x[2]
if __name__=='__main__':
    zp,B=setup()
    print("z_p(C)", mp.nstr(abs(zp),5), " |B|", mp.nstr(abs(B),8), " shallow scale for width w ~ |B|^2 w^2", flush=True)
    random.seed(1)
    for w in [mp.mpf(s) for s in sys.argv[1:]]:
        pts=[]
        for k in range(160):
            x=random.uniform(-0.5,0.5); y=random.uniform(-0.28,0.28)
            pts.append((mp.mpc(x*w, y*w), B, w))
        with Pool(18) as pool: res=pool.map(work, pts)
        ok=[r for r in res if r[2] is not None]
        both_in=sum(1 for r in res if r[0] and r[1]); mism=sum(1 for r in res if r[0]!=r[1])
        disp=sorted(r[3] for r in ok); dn=sorted(abs(r[2]) for r in ok)
        m=len(ok)
        it_deep=sum(r[4] for r in ok)/max(m,1); it_sh=sum(r[5] for r in ok)/max(m,1)
        print(f"w={mp.nstr(w,3):8} pts {len(res)} class-mismatch {mism} both-bounded {both_in} | |dnu| p50 {dn[m//2]:.1e} max {dn[-1]:.1e} | displacement px p50 {disp[m//2]:.1e} p95 {disp[int(.95*m)]:.1e} max {disp[-1]:.1e} | iters deep {it_deep:.0f} vs shallow {it_sh:.0f} | shallow |d'|~{mp.nstr(abs(B)**2*w*w/4,3)}", flush=True)
