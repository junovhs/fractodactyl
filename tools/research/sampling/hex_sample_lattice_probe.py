# Probe: at equal samples/pixel, does a triangular (hex) sample lattice beat fd's square ss grid?
# Truth = same Gaussian reconstruction from a 12x12-per-pixel grid. Error = RMS / p99.9 of shaded value (0..1).
import numpy as np, sys, time
W,H=320,180
def nu(c, maxit):
    z=np.zeros_like(c); out=np.zeros(c.shape); alive=np.ones(c.shape,bool); idx=np.arange(c.size)
    zf=z.ravel(); cf=c.ravel(); of=out.ravel()
    for n in range(maxit):
        zf=zf*zf+cf
        a=zf.real**2+zf.imag**2
        esc=a>1e6
        if esc.any():
            of[idx[esc]]=n+1-np.log2(np.log2(a[esc])/2)
            keep=~esc; zf=zf[keep]; cf=cf[keep]; idx=idx[keep]
        if idx.size==0: break
    of[idx]=-1.0
    return out
def shade(v, dens):
    s=0.5+0.5*np.cos(2*np.pi*dens*v)
    s[v<0]=0.0
    return s
def render(px,py,view,dens,maxit,sig):
    if px.size>1500000:
        parts=np.array_split(np.arange(px.size),int(px.size/1000000)+1)
        N=np.zeros((H,W));D=np.zeros((H,W))
        for q in parts:
            n_,d_=render1(px[q],py[q],view,dens,maxit,sig); N+=n_; D+=d_
        return N/D
    n_,d_=render1(px,py,view,dens,maxit,sig); return n_/d_
def render1(px,py,view,dens,maxit,sig):
    cre,cim,width=view; h=width/W
    c=(cre+(px-W/2)*h)+1j*(cim-(py-H/2)*h)
    f=shade(nu(c,maxit),dens)
    num=np.zeros((H,W)); den=np.zeros((H,W))
    ix=np.floor(px).astype(int); iy=np.floor(py).astype(int); R=int(np.ceil(3*sig))
    for dy in range(-R,R+1):
        for dx in range(-R,R+1):
            qx=ix+dx; qy=iy+dy
            ok=(qx>=0)&(qx<W)&(qy>=0)&(qy<H)
            w=np.exp(-((px-(qx+0.5))**2+(py-(qy+0.5))**2)/(2*sig*sig))
            np.add.at(num,(qy[ok],qx[ok]),(w*f)[ok]); np.add.at(den,(qy[ok],qx[ok]),w[ok])
    return num,den
def lattice(a1,a2,off,m=4):
    A=np.array([[a1[0],a2[0]],[a1[1],a2[1]]]); Ai=np.linalg.inv(A)
    cs=np.array([[-m,-m],[W+m,-m],[-m,H+m],[W+m,H+m]],float)-np.array(off)
    ij=cs@Ai.T
    i0,j0=np.floor(ij.min(0)).astype(int)-1; i1,j1=np.ceil(ij.max(0)).astype(int)+1
    i,j=np.meshgrid(np.arange(i0,i1+1),np.arange(j0,j1+1))
    x=i*a1[0]+j*a2[0]+off[0]; y=i*a1[1]+j*a2[1]+off[1]
    k=(x>-m)&(x<W+m)&(y>-m)&(y<H+m)
    return x[k],y[k]
def square(spp,off,rot=0.0):
    s=1/np.sqrt(spp); c,sn=np.cos(rot),np.sin(rot)
    return lattice((s*c,s*sn),(-s*sn,s*c),off)
def hexl(spp,off):
    s=np.sqrt(2/(np.sqrt(3)*spp))
    return lattice((s,0),(s/2,s*np.sqrt(3)/2),off)
def jitter(spp,rng):
    x,y=square(spp,(0,0)); s=1/np.sqrt(spp)
    return x+rng.uniform(-s/2,s/2,x.shape), y+rng.uniform(-s/2,s/2,y.shape)
views={'seahorse 4e-3':(-0.7453,0.1127,4e-3,3000),'seahorse 2.5e-5':(-0.74529,0.113075,2.5e-5,6000),'elephant 1e-3':(0.2826,0.0100,1e-3,4000)}
sig=float(sys.argv[1]) if len(sys.argv)>1 else 0.5
rng=np.random.default_rng(1)
for name,(cre,cim,wd,mi) in views.items():
  for dens in (0.25,1.0):
    view=(cre,cim,wd)
    tx,ty=square(100,(1/20,1/20))
    truth=render(tx,ty,view,dens,mi,sig)
    core=(slice(4,H-4),slice(4,W-4))
    def score(gen,reps=3):
        r=[];p=[]
        for k in range(reps):
            x,y=gen(k); e=(render(x,y,view,dens,mi,sig)-truth)[core]
            r.append(np.sqrt((e**2).mean())); p.append(np.quantile(abs(e),0.999))
        return np.mean(r),np.mean(p),x.size/((W+8)*(H+8))
    offs=[rng.uniform(0,1,2) for _ in range(3)]
    rows=[('square 4.00 (fd --ss 2)',lambda k:square(4,offs[k]*0.5)),
          ('square rot 26.6deg 4.00',lambda k:square(4,offs[k]*0.5,0.4636)),
          ('jittered 4.00',lambda k:jitter(4,rng)),
          ('hex 4.00',lambda k:hexl(4,offs[k]*0.5)),
          ('hex 3.46 (-13.4%)',lambda k:hexl(4*np.sqrt(3)/2,offs[k]*0.5)),
          ('hex 3.00 (-25%)',lambda k:hexl(3,offs[k]*0.5)),
          ('square 3.06 (1.75^2)',lambda k:square(3.0625,offs[k]*0.5))]
    print(f'## {name}, band density {dens}, sigma {sig}px'); base=None
    for lab,g in rows:
        r,p,spp=score(g)
        base=base or r
        print(f'  {lab:26s} spp {spp:4.2f}  rms {r:.5f} ({r/base:5.2f}x)  p99.9 {p:.4f}',flush=True)
