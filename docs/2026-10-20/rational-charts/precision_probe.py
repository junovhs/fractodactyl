#!/usr/bin/env python3
"""Attribute valley error to reference conversion vs ordinary arithmetic."""
import json,time
from pathlib import Path
import mpmath as mp
import numpy as np
from tournament import HERE,SCENES,previous,save_csv


def run(out):
    started=time.perf_counter();truth=json.loads((HERE.parent/'truth.json').read_text())
    targets=[r for r in truth['rows'] if r['scene']=='valley-14'];_,re,im,w=SCENES['valley-14'];limit=max(r['n'] for r in targets)
    with mp.workdps(100):
        C=mp.mpc(re,im);rho=mp.mpf(w);hp=[mp.mpc(0)]
        for n in range(limit):hp.append(hp[-1]**2+C)
        ref=[complex(z) for z in hp];rows=[]
        for target in targets:
            dc=mp.mpc(*target['x'])*rho;zt=mp.mpc(*target['z']);qt=mp.mpc(*target['q'])
            for mode in ['binary64','longdouble','mp-rounded-reference','mp-split-reference','mp-exact-reference']:
                t=time.perf_counter();u=0j;q=0j
                if mode.startswith('mp-'):u=mp.mpc(0);q=mp.mpc(0)
                elif mode=='longdouble':u=np.clongdouble(0);q=np.clongdouble(0)
                for n in range(target['n']):
                    if mode=='binary64':Z=ref[n];offset=complex(dc);step_rho=float(rho)
                    elif mode=='longdouble':
                        # Parse original high-precision reference decimal into extended precision.
                        Z=np.clongdouble(np.longdouble(mp.nstr(hp[n].real,30))+np.clongdouble(1j)*np.longdouble(mp.nstr(hp[n].imag,30)))
                        offset=np.clongdouble(np.longdouble(mp.nstr(dc.real,30))+np.clongdouble(1j)*np.longdouble(mp.nstr(dc.imag,30)));step_rho=np.longdouble(w)
                    elif mode=='mp-rounded-reference':Z=mp.mpc(ref[n]);offset=dc;step_rho=rho
                    elif mode=='mp-split-reference':Z=mp.mpc(ref[n])+(hp[n]-mp.mpc(ref[n]));offset=dc;step_rho=rho
                    else:Z=hp[n];offset=dc;step_rho=rho
                    q=2*(Z+u)*q+step_rho;u=u*(2*Z+u)+offset
                final=hp[target['n']]+mp.mpc(str(u.real),str(u.imag))
                qm=mp.mpc(str(q.real),str(q.imag))
                rows.append(dict(x=str(target['x']),mode=mode,n=target['n'],state_error=float(abs(final-zt)),derivative_relative_error=float(abs(qm-qt)/abs(qt)),elapsed_s=time.perf_counter()-t))
            print('precision',target['x'],flush=True)
    out.mkdir(parents=True,exist_ok=True);save_csv(out/'precision.csv',rows)
    (out/'precision-run.json').write_text(json.dumps(dict(elapsed_s=time.perf_counter()-started,longdouble_bits=int(np.finfo(np.longdouble).nmant+1),note='Diagnostic arithmetic attribution, no performance claim; final index fixed by frozen truth'),indent=2)+'\n')
    for r in rows:print(r)


if __name__=='__main__':run(HERE/'results/precision')
