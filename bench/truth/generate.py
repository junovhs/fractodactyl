"""TRUT-01 independent oracle pack generator. No production-renderer inputs.

Direct iteration/analytic component tests come from tools/oracle.py. Derived values
and rates use that oracle's documented definitions; precision is depth+160 bits.
"""
import argparse
from concurrent.futures import ProcessPoolExecutor
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import os
import sys
import time
import mpmath
from mpmath import mp

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('oracle', ROOT/'tools/oracle.py')
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)


def cases():
    v0 = next(l.split() for l in (ROOT/'bench/path-atlas-v0.txt').read_text().splitlines() if l and not l.startswith('#'))
    return [dict(name=n, re=r, im=i, width=w, nx=nx, ny=ny, cap=cap) for n,r,i,w,nx,ny,cap in [
        ('exterior', '0.5', '0.5', '0.1',32,32,200),
        ('filament', '-0.7453', '0.1127', '0.0065',32,32,2000),
        ('period764', v0[0],v0[1], '2e-49',32,32,8000),
        ('parabolic', '0.25','0','1e-3',1024,1,2000),
        ('extreme', '0','1','1e-300',32,32,4000)]]


def point(task):
    c, idx, guard = task
    mp.prec = 64
    depth = max(0, -int(math.floor(float(mp.log(mp.mpf(c['width'])/c['nx'], 2)))))
    mp.prec = depth + guard
    h = mp.mpf(c['width'])/c['nx']
    x,y = idx % c['nx'],idx // c['nx']
    cr = mp.mpf(c['re']) + h*(mp.mpf(x)+mp.mpf('.5')-mp.mpf(c['nx'])/2)
    ci = mp.mpf(c['im']) - h*(mp.mpf(y)+mp.mpf('.5')-mp.mpf(c['ny'])/2)
    if oracle.in_main_components(cr,ci):
        return [idx,'A',0,0,0,0,0,0]
    result = oracle.direct(cr,ci,c['cap'],mp.mpf('1e20'))
    if result is None:
        return [idx,'U',c['cap'],0,0,0,0,0]
    n,zr,zi,dr,di,sr,si = result
    az = mp.sqrt(zr*zr+zi*zi)
    nu = n+1-mp.log(mp.log(az,2),2)
    de = 2*az*mp.log(az)/mp.sqrt(dr*dr+di*di)/h
    m2,d2 = zr*zr+zi*zi,dr*dr+di*di
    gr,gi = (zr*dr+zi*di)/m2,(zr*di-zi*dr)/m2
    hr,hi = (sr*dr+si*di)/d2,(si*dr-sr*di)/d2
    f = 1+1/mp.log(az)
    de_rate = de*mp.sqrt((gr*f-hr)**2+(gi*f-hi)**2)*h
    normal_rate = mp.sqrt((gr-hr)**2+(gi-hi)**2)*h
    ang = mp.atan2(-(zi*dr-zr*di),zr*dr+zi*di)
    values = [float(v) for v in [nu,de,ang,de_rate,normal_rate]]
    if not all(math.isfinite(v) for v in values):
        raise ValueError((c['name'],idx,'nonfinite oracle output'))
    return [idx,'E',n,*values]


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--out',type=Path,required=True)
    ap.add_argument('--workers',type=int,default=1)
    ap.add_argument('--validate',action='store_true')
    a=ap.parse_args()
    workers=min(a.workers,int(os.environ.get('ISHOO_CPU_ALLOCATION',a.workers)))
    if workers<1: raise ValueError('positive worker lease required')
    a.out.mkdir(parents=True,exist_ok=True)
    provenance=dict(schema='fd-truth/1',oracle_sha256=hashlib.sha256((ROOT/'tools/oracle.py').read_bytes()).hexdigest(),generator_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),mpmath=mpmath.__version__,python=sys.version.split()[0],guard_bits=160,escape_radius='1e10',ss=1,rotation=0)
    started=time.perf_counter()
    with ProcessPoolExecutor(max_workers=workers) as pool:
        for c in cases():
            path=a.out/(c['name']+'.tsv')
            if a.validate:
                rows={int(l.split('\t')[0]):l.split('\t') for l in path.read_text().splitlines() if l and l[0].isdigit()}
                # Fixed indices spread over every row; independent higher precision
                # confirms the stored binary64 observations, not production output.
                indices=list(range(0,1024,32))+[1023]
                checked=list(pool.map(point,[(c,i,416) for i in indices],chunksize=1))
                for row in checked:
                    old=rows[row[0]]
                    assert old[1]==row[1] and int(old[2])==row[2],(c['name'],row[0],'class/escape iteration')
                    for x,y in zip(old[3:],row[3:]):
                        assert math.isclose(float(x),float(y),rel_tol=2e-14,abs_tol=2e-14),(c['name'],row[0],x,y)
                print(json.dumps(dict(name=c['name'],higher_precision_points=len(checked))),flush=True)
                continue
            t=time.perf_counter()
            rows=list(pool.map(point,[(c,i,160) for i in range(1024)],chunksize=8))
            mp.prec=64
            bits=max(0,-int(math.floor(float(mp.log(mp.mpf(c['width'])/c['nx'],2)))))+160
            header=['fd-truth\t1', '\t'.join(map(str,['view',c['name'],c['re'],c['im'],c['width'],c['nx'],c['ny'],1,c['cap'],'1e10',0,bits])), '# index\tclass(E escaped/A analytic component/U finite-cap nonescape)\tescape_iteration\tnu\tde_px\tnormal_rad\tde_rate_per_px\tnormal_rate_per_px']
            path.write_text('\n'.join(header+['\t'.join(map(str,r)) for r in rows])+'\n')
            print(json.dumps(dict(name=c['name'],seconds=time.perf_counter()-t,classes={k:sum(r[1]==k for r in rows) for k in ['E','A','U']})),flush=True)
    if not a.validate:
        (a.out/'provenance.json').write_text(json.dumps(provenance,indent=2,sort_keys=True)+'\n')
    print(json.dumps(dict(seconds=time.perf_counter()-started,workers=workers)),flush=True)

if __name__=='__main__':main()
