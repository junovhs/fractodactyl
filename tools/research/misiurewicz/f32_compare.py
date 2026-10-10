import sys, numpy as np
from PIL import Image
nu=np.load(sys.argv[1]); out=sys.argv[2]
def shade(v,dens):
    t=v*dens
    stops=np.array([[8,20,48],[40,110,190],[200,235,255],[255,255,255],[120,170,220],[8,20,48]],float)   # icy loop
    x=(t%1.0)*(len(stops)-1); i=np.floor(x).astype(int); f=(x-i)[...,None]
    rgb=stops[i]*(1-f)+stops[i+1]*f
    line=np.abs((t*4)%1.0-0.5)>0.46                      # thin terrain lines, 4 per band
    rgb[line]*=0.35
    rgb[v<0]=0
    return rgb
for dens in (0.05,0.25):
    a,b=shade(nu[0],dens),shade(nu[1],dens)
    d=np.abs(a-b).max(-1); n=d.size
    print(f'band density {dens}: pixels identical after 8-bit rounding {np.mean(np.round(a)==np.round(b)):.4%} of channels;'
          f' pixels off by >1 level {np.sum(d>1)}/{n}, >4 levels {np.sum(d>4)}/{n}, >16 levels {np.sum(d>16)}/{n}; max diff {d.max():.0f}/255')
    if dens==0.05:
        up=lambda im: np.kron(im,np.ones((4,4,1)))
        amp=np.clip(d[...,None]*16,0,255)*np.ones(3)
        sheet=np.concatenate([up(a),up(b),up(amp)],1).astype(np.uint8)
        Image.fromarray(sheet).save(out)
dn=np.abs(nu[0]-nu[1]); ok=(nu[0]>=0)&(nu[1]>=0)
print(f'nu difference: median {np.median(dn[ok]):.1e}, p99 {np.quantile(dn[ok],0.99):.1e}, max {dn[ok].max():.1e}; escaped in both {ok.mean():.2%}')
