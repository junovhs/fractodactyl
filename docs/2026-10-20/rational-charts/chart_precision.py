#!/usr/bin/env python3
"""Replay rejected narrow charts at the identical prefix with better build arithmetic.
No new guard or prefix selection: isolates reference and coefficient rounding.
"""
import csv,json,time
import mpmath as mp
import numpy as np
from tournament import HERE,save_csv
from guard_probe import onepole
from local_probe import evaluate


def run():
    started=time.perf_counter();truth=json.loads((HERE/'local-truth.json').read_text())
    configs=[(r['method'],int(r['order']),int(r['denom']),int(r['prefix'])) for r in csv.DictReader((HERE/'results/radius/local-timings.csv').open()) if r['method']!='plain']
    limit=max(c[3] for c in configs);rho=mp.mpf('1e-8');scale=.21;center=.5
    with mp.workdps(100):
        z=mp.mpc(0);C=mp.mpf('.25')+mp.mpf('.5')*mp.mpf('1e-8');hp=[z]
        for _ in range(limit):z=z*z+C;hp.append(z)
        ws=[complex(z-mp.mpf('.5')) for z in hp]
        ref64=[complex(z) for z in hp]
        ref80=[np.clongdouble(np.longdouble(mp.nstr(z.real,30))+np.clongdouble(1j)*np.longdouble(mp.nstr(z.imag,30))) for z in hp]
    rows=[]
    for mode,dtype,ref in [('f64-ref64',np.complex128,ref64),('f80-ref64',np.clongdouble,ref64),('f80-ref80',np.clongdouble,ref80)]:
        b=np.zeros(17,dtype=dtype);stored={}
        for n in range(1,limit+1):
            b=(2*dtype(ref[n-1])*b+np.convolve(b,b)[:17]).astype(dtype)
            # Construct scale in decimal at the build precision, rather than rounding its product first.
            b[1]+=dtype(np.longdouble('0.21')*np.longdouble('1e-8')) if dtype==np.clongdouble else scale*1e-8
            if n in [c[3] for c in configs]:stored[n]=b.copy()
        for name,order,m,n in configs:
            bb=stored[n][:order+1]
            if m:
                Q=np.array([1,-bb[-1]/bb[-2]],dtype=dtype);P=np.convolve(bb,Q)[:len(bb)-1]
            else:P=bb;Q=[1+0j]
            init=(n,list(map(complex,P)),list(map(complex,Q)),0.,0.)
            errors=0;maxq=0.;maxnu=0.
            for target in truth['rows']:
                x=complex(*map(float,target['x']));result=evaluate(ws,x*1e-8,(x-center)/scale,scale,init,100000)
                nn,escaped,z,q,ordinary=result;zt=complex(*map(float,target['z']));qt=complex(*map(float,target['q']))
                qe=abs(q-qt)/abs(qt);nu=abs(np.log2(np.log(abs(z)))-np.log2(np.log(abs(zt))))
                ok=nn==target['n'] and escaped==target['escaped'] and abs(z-zt)<1e-6*max(1,abs(zt)) and qe<1e-6 and nu<1e-6
                errors+=not ok;maxq=max(maxq,qe);maxnu=max(maxnu,nu)
            rows.append(dict(method=name,mode=mode,prefix=n,errors=errors,max_derivative_relative_error=maxq,max_smooth_error=maxnu))
            print(rows[-1],flush=True)
    save_csv(HERE/'results/chart-precision.csv',rows)
    (HERE/'results/chart-precision-run.json').write_text(json.dumps(dict(elapsed_s=time.perf_counter()-started,note='Identical rejected prefixes; higher build precision only; binary64 evaluation/tail; no certified guard'),indent=2)+'\n')


if __name__=='__main__':run()
