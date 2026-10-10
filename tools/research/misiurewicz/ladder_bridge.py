# Probe H: does the number of minibrot periods a pixel spends before escaping grow like log2(n)
# down the ladder (the "bridge" of the universal-landing answer, eq. B3), not stay flat?
# usage: python bridge.py   (reads ladder_rungs.txt next to it)
import os, random, math, time
from mpmath import mp, mpf, mpc
here=os.path.dirname(os.path.abspath(__file__))
R={}
for line in open(os.path.join(here,'ladder_rungs.txt')):
    if line.startswith('#') or not line.strip(): continue
    k,P,re,im=line.split(); R[int(k)]=(int(P),re,im)
RE0='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
IM0='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
SIZE0=4.1205e-50; LOGRHO=math.log(1.153243328)
rows=[(0,764,RE0,IM0)]+[(k,)+R[k] for k in (409,7676)]
random.seed(9); offs=[complex(random.uniform(-.5,.5),random.uniform(-.5,.5)) for _ in range(6)]
print('frame 5,000 minibrot sizes wide, 6 pixels (same relative positions at every rung)')
base=None
for k,P,re,im in rows:
    n=(P-24)//2 - 1                                   # laps index on this ladder: P = 24 + 2n + 2
    logsize=math.log10(SIZE0)-2*k*LOGRHO/math.log(10)
    mp.dps=int(-logsize)+45; t=time.time()
    cn=mpc(mpf(re),mpf(im)); size=mpf(10)**logsize; per=[]
    for o in offs:
        c=cn+size*5000*mpc(o.real,o.imag); z=mpc(0); s=0
        while abs(z)<1e3 and s<40*P: z=z*z+c; s+=1
        per.append(s/P)
    m_=sum(per)/len(per); base=base or (m_,n)
    print(f'  rung k={k:5d}: period {P:6d}, n={n:5d}, size 1e{logsize:.0f}: periods before escape {min(per):.2f}-{max(per):.2f} (mean {m_:.2f});'
          f' change vs v0 {m_-base[0]:+.2f}, log2(n/n0) = {math.log2(n/base[1]):+.2f}   [{time.time()-t:.0f} s]',flush=True)
