import mpmath as mp, math, sys, random
from multiprocessing import Pool
P=764; L=24; DPS=110; MAXIT=400000; R2=mp.mpf(10)**20
mp.mp.dps=DPS
C=mp.mpc('-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502','0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922')
def consts():
    mp.mp.dps=DPS
    c0=mp.mpc('-0.74329189085243020293162432597251075717634855812077658183233','0.131240552308797604770845906581478143077114151158584006731334')
    for it in range(80):
        z=mp.mpc(0); dz=mp.mpc(0); zs=[]; dzs=[]
        for n in range(L+3): zs.append(z); dzs.append(dz); dz=2*z*dz+1; z=z*z+c0
        st=(zs[L+2]-zs[L])/(dzs[L+2]-dzs[L]); c0-=st
        if abs(st)<mp.mpf(10)**-(DPS-5): break
    z=mp.mpc(0); dz=mp.mpc(0); zs=[]; dzs=[]
    for n in range(L+3): zs.append(z); dzs.append(dz); dz=2*z*dz+1; z=z*z+c0
    alpha, beta = zs[L], zs[L+1]
    rho=4*alpha*beta
    dalpha=(2*beta+1)/(1-rho)
    Dp = dzs[L]-dalpha                  # d/dc (z_L(c) - alpha(c)) at c0
    D = mp.mpc(1)
    for i in range(1,L): D*=2*zs[i]     # (f^{L-1})'(z_1=c0): z_1 -> z_L
    return c0, D/Dp, rho
def nu(c, z0=None, n0=0):
    mp.mp.dps=DPS
    z=mp.mpc(0) if z0 is None else z0
    dz=mp.mpc(0)
    for n in range(n0, MAXIT):
        dz=2*z*dz+1; z=z*z+c
        a=z.real*z.real+z.imag*z.imag
        if a>R2:
            az=mp.sqrt(a)
            return float(n+1-mp.log(mp.log(az))/mp.log(2)), 2*az*mp.log(az)/abs(dz), n+1
    return None, None, MAXIT
def work(args):
    mp.mp.dps=DPS
    d, w, c0, K = args
    c=C+d
    a=nu(c)
    # exact orbit to step P+1
    z=mp.mpc(0)
    for i in range(P+1): z=z*z+c
    cp = c0 + (z - c0)*K
    b=nu(cp)
    if a[0] is None or b[0] is None: return (a[0] is None, b[0] is None, None, None, a[2], b[2])
    dnu=a[0]-b[0]
    disp=abs(dnu - 0)*math.log(2)*float(a[1]/(w/480))
    return (False, False, dnu, disp, a[2], b[2], float(a[1]/(w/480)))
if __name__=='__main__':
    c0,K,rho=consts()
    print("K = D/Delta' =", mp.nstr(K,10), "|K|", mp.nstr(abs(K),6), flush=True)
    random.seed(1)
    for w in [mp.mpf(s) for s in sys.argv[1:]]:
        pts=[(mp.mpc(random.uniform(-.5,.5)*w, random.uniform(-.28,.28)*w), w, c0, K) for k in range(160)]
        with Pool(18) as pool: res=pool.map(work, pts)
        ok=[r for r in res if r[2] is not None]; m=len(ok)
        mism=sum(1 for r in res if r[0]!=r[1])
        dn=sorted(r[2] for r in ok); off=dn[m//2]
        disp=sorted(abs(r[2]-off)*math.log(2)*r[6] for r in ok)
        print(f"w={mp.nstr(w,3):8} class-mismatch {mism}/{len(res)} | nu offset median {off:.6f} | |nu-off| as px p50 {disp[m//2]:.1e} p95 {disp[int(.95*m)]:.1e} max {disp[-1]:.1e} | iters deep {sum(r[4] for r in ok)/m:.0f} shallow {sum(r[5] for r in ok)/m:.0f}", flush=True)
