import json,math,struct,collections
from pathlib import Path
base=Path('/home/juno/fractodactyl/target/gpu32-attribution')
names=['seahorse','cusp','seahorse-deep','valley','exterior','tim-random']
frames=[]
def guard(v,pm,pe):
 k,m,n,status,reb,firstn,firstm,firsttype,term,fq,fe,fd,er,az,ez,ad,ed,dexp,r=v
 if status!=1:return 'range-or-invalid' if status>=2 else 'step-budget'
 if not all(math.isfinite(x) for x in [az,ez,ad,ed]) or ez<0 or ed<0:return 'nonfinite-bound'
 if az-ez<=r:return 'escape-ambiguous'
 if ed>=ad/2:return 'derivative-bound'
 lo=az-ez;lnz=math.log(az);llo=math.log(lo);lhi=math.log(az+ez)
 nu=math.log1p(-math.log1p(-ez/az)/llo)/math.log(2)+8*2**-53*(n+8)
 g=math.ldexp(lhi,-int(n))
 de_lo=(lo*llo)/(az*lnz)*ad/(ad+ed)*math.exp(-g)*(1-4/(lo*lo))*(1-1e-6)
 de_hi=((az+ez)*lhi)/(az*lnz)*ad/(ad-ed)*(1+1e-6)
 bound=nu*de_hi/de_lo*(1+1e-6)
 de=math.ldexp(2*az*lnz/(ad*pm),-int(dexp+pe))
 try:de=struct.unpack('f',struct.pack('f',de))[0]
 except OverflowError:de=math.inf
 displacement=bound*de*de_lo*math.log(2)/2
 if not math.isfinite(displacement) or displacement>5e-4:return 'orbit-displacement-bound'
 hi=(az+ez)*math.log(az+ez)/(az*lnz)*ad/(ad-ed)
 low=(az-ez)*math.log(az-ez)/(az*lnz)*ad/(ad+ed)
 if not math.isfinite(de) or hi-1+2**-23>1e-3 or 1-low+2**-23>1e-3:return 'de-relative-bound'
 angle=math.asin(ez/az)+math.asin(ed/ad)
 if angle+math.tau/65536>math.radians(.1):return 'normal-bound'
 return 'accepted'
rows=[]
for line in (base/'states.tsv').read_text().splitlines():
 if line.startswith('FRAME'):
  if rows:frames.append((header,rows))
  header=line.split('\t')[1:];rows=[]
 else:rows.append(list(map(float,line.split('\t'))))
frames.append((header,rows));out=[]
for name,(h,rows) in zip(names,frames):
 counts=collections.Counter(guard(v,float(h[5]),int(h[6])) for v in rows)
 rebase=collections.Counter('none' if v[5]==0 else 'reference-end' if v[7]==1 else 'cancellation' for v in rows)
 statuses=collections.Counter(int(v[3]) for v in rows)
 reps=[]
 for reason in counts:
  v=next(v for v in rows if guard(v,float(h[5]),int(h[6]))==reason)
  reps.append(dict(reason=reason,x=int(v[0])%128,y=int(v[0])//128,m=int(v[1]),n=int(v[2]),rebases=int(v[4]),first_rebase_n=int(v[5]),first_rebase_m=int(v[6]),first_rebase_type=int(v[7]),termination_code=int(v[8]),first_upload_radius=v[9],error_before_first_rebase=v[10],delta_before_first_rebase=v[11],propagated_rebase_contribution=v[12],az=v[13],ez=v[14],ad=v[15],ed=v[16],derivative_exponent=int(v[17])))
 out.append(dict(name=name,camera=h[:3],bits=int(h[3]),reference_len=int(h[4]),first_reject_counts=counts,first_rebase_types=rebase,statuses=statuses,rebased_before_budget=sum(v[5]>0 for v in rows),rebase_dominates_final_e=sum(v[12]>.5*v[14] for v in rows if v[14]>0),n_range=[min(v[2] for v in rows),max(v[2] for v in rows)],representatives=reps))
(base/'summary.json').write_text(json.dumps(out,indent=2)+'\n')
for r in out: print(json.dumps(r))
