#!/usr/bin/env python3
"""BLA one-step radius correction vs naive eta|A|, same inputs/reference."""
import csv,math,collections,statistics
import mpmath as mp
from fractal_la_experiment import CASES,WIDTHS,OFFSETS,N,hp_reference,hp_pixel,bla,EscapeGuard

def standard_bla(ref,width,eta):
 n=len(ref)-1;levels=collections.defaultdict(list); prev=[]
 for j in range(n):
  a=2*ref[j]; m=abs(a)
  r=max(0.,eta*m-width/m) if m>0 else 0.
  elem=(j,j+1,a,1+0j,r,abs(ref[j+1]));prev.append(elem);levels[j].append(elem)
 step=1
 while 2*step<=n:
  curr=[]
  for p in range(0,len(prev)-1,2):
   x,y=prev[p:p+2]
   if x[1]!=y[0]:continue
   a=y[2]*x[2]; b=y[2]*x[3]+y[3]
   ry=y[4]-abs(x[3])*width
   r=max(0.,min(x[4],ry/abs(x[2]))) if abs(x[2])>0 else x[4] if ry>0 else 0.
   elem=(x[0],y[1],a,b,r,max(x[5],y[5]));curr.append(elem);levels[x[0]].append(elem)
  prev=curr; step*=2
 return levels

if __name__=='__main__':
 mp.mp.dps=100;rows=[]
 for name,(re,im) in CASES.items():
  hp=hp_reference(mp.mpc(mp.mpf(re),mp.mpf(im)),N);ref=[complex(x) for x in hp]
  for w in WIDTHS:
   rho=10.**(-w)
   for eta in [.005,.05,2**-24]:
    lookup=standard_bla(ref,rho,eta)
    for ox,oy in OFFSETS:
     dchp=mp.mpc(str(ox),str(oy))*mp.power(10,-w);dc=complex(dchp)
     _,_,hp_n=hp_pixel(hp,dchp,N)
     u,d,n,stat=bla(ref,dc,lookup,EscapeGuard(hp,ref))
     rows.append(dict(case=name,width=w,eta=eta,ordinary=stat['ordinary'],skipped=stat['skip'],blocks=stat['blocks'],tested=stat['candidates'],rejected=stat['rejected'], correct=int(n==hp_n),e_ref=hp_n,e_act=n))
  print(name,'finished',flush=True)
 with open('/mnt/data/fractal_la_bla_radius.csv','w',newline='') as f:
  dw=csv.DictWriter(f,fieldnames=list(rows[0]));dw.writeheader();dw.writerows(rows)
 for eta in [.005,.05,2**-24]:
  x=[r for r in rows if r['eta']==eta]
  print('eta',eta,'avg ord',statistics.mean(r['ordinary'] for r in x),'avg skips',statistics.mean(r['skipped'] for r in x),'escape agreement',sum(r['correct'] for r in x),len(x),'reject rate',sum(r['rejected'] for r in x)/max(1,sum(r['tested'] for r in x)),flush=True)
