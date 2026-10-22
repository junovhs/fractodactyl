#!/usr/bin/env python3
"""Independent highprecision sanity check of the analytically bounded Float reference."""
import argparse
import json
from pathlib import Path
import mpmath as mp
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--input',type=Path,required=True);a=p.parse_args()
reports=[];row=None
for line in a.input.read_text().splitlines():
    f=line.split('|')
    if f[0]=='camera':
        if row:reports.append(row)
        name,re,im,bits,radius=f[1:];mp.mp.prec=int(bits)+768
        c=mp.mpc(re,im);z=mp.mpc(0);n=0
        row=dict(name=name,bits=int(bits),points=0,worst_error_radius_ratio=0.0,nonfinite_radii=0)
    else:
        index=int(f[1]);assert index==n
        if n:z=z*z+c
        # Compare the exact stored binary64 values, not their decimal spellings.
        x,y,q=(float(v) for v in f[2:]);error=abs(z-mp.mpc(mp.mpf(x),mp.mpf(y)))
        assert error<=mp.mpf(q),(row['name'],n,mp.nstr(error),q)
        if mp.isfinite(q) and q>0:row['worst_error_radius_ratio']=max(row['worst_error_radius_ratio'],float(error/q))
        elif not mp.isfinite(q):row['nonfinite_radii']+=1
        row['points']+=1;n+=1
if row:reports.append(row)
a.input.with_suffix('.json').write_text(json.dumps(reports,indent=2)+'\n')
print(json.dumps(reports))
