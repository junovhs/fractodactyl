#!/usr/bin/env python3
"""Independent algebra checks and frozen positive/negative research controls."""
import csv,json,math
import numpy as np
from shadow_probe import HERE,comparison,g_lower
from tournament import value

rng=np.random.default_rng(1040)
for trial in range(100):
    b=np.r_[0j,(rng.normal(size=32)+1j*rng.normal(size=32))*np.power(.4,np.arange(1,33))*.001]
    E=1e-12;F=1e-11
    P,Q,e,f,q=comparison(b,E,F,8,True)
    if q<=0:continue
    for k in range(8):
        x=.95*np.exp(2j*np.pi*k/8)
        u=np.polynomial.polynomial.polyval(x,b)
        du=np.polynomial.polynomial.polyval(x,np.arange(1,len(b))*b[1:])
        r,dr,_=value(P,Q,x)
        assert abs(u-r)+E<=e+1e-12
        assert abs(du-dr)+F<=f+1e-12
        # Growing the truncation order by appending zeros changes no represented state/derivative.
        padded=np.pad(b,(0,32))
        assert abs(np.polynomial.polynomial.polyval(x,padded)-u)<1e-15

# A thin rectangle can retain a flow-denominator bound after the enclosing disk loses it.
eta=1e-5/.21;a=.2/.21;rho=.21e-8;eps=.5e-8
for trial in range(100):
    b=np.r_[0.,rng.normal(size=16)*np.power(.5,np.arange(1,17))*1e-5].astype(complex)
    W=b.copy();W[0]=5e-5;E=1e-12
    norm=lambda v:float(np.sum(np.abs(v)))
    dn=lambda v:float(np.sum(np.arange(1,len(v))*np.abs(v[1:])))
    lower=max(0.,3e-9-eta*(2*norm(W)*dn(W)+rho)-2*norm(W)*E-E*E)
    for _ in range(8):
        x=rng.uniform(-a,a)+1j*rng.uniform(-eta,eta)
        error=E*np.exp(2j*np.pi*rng.random())
        w=np.polynomial.polynomial.polyval(x,W)+error
        assert abs(w*w+eps+rho*x)>=lower-1e-20

# Full-disk g bound must include the rho*x forcing term (regression for derivation).
b=np.array([0j,1e-6+0j]);Z=.50001;E=1e-15
lower=g_lower(b,Z,eps,rho,E)
for t in np.linspace(0,2*np.pi,100):
    x=np.exp(1j*t);w=Z-.5+np.polynomial.polynomial.polyval(x,b)
    assert abs(w*w+eps+rho*x)>=lower-1e-20

rows=list(csv.DictReader((HERE/'results/K64-strip/training.csv').open()))
assert next(r for r in rows if r['method']=='p8-r0-phase-ref')['errors']=='2'
assert all(r['errors']=='0' for r in rows if r['rule']=='phase-strip')
for file in ['native-growth-holdout.csv','cold-training-growth.csv','cold-holdout-growth.csv','cold-fresh-growth.csv','native-fresh.csv']:
    rr=list(csv.DictReader((HERE/'results'/file).open()))
    assert all(r['errors']=='0' for r in rr)
native=list(csv.DictReader((HERE/'results/native-growth-holdout.csv').open()))
base=next(r for r in native if r['method']=='K64-P64')
adapt=next(r for r in native if r['method']=='A64-g4')
assert adapt['prefix']==base['prefix']=='34720'
assert int(base['coefficient_pairs'])==36000*64*65//2
assert int(adapt['coefficient_pairs'])<int(base['coefficient_pairs'])//2
valley=list(csv.DictReader((HERE/'results/band-valley-14.csv').open()))
assert all(int(r['errors'])>0 for r in valley)
interval=list(csv.DictReader((HERE/'results/reference-interval.csv').open()))
assert all(float(r['actual_error'])<=float(r['reference_radius']) for r in interval)
print('Shadow rational bounds, zero-extension identities, disk/strip flow bounds passed')
print('Native/cold positive controls, phase/valley failures, adaptive work counts preserved')
