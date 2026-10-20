#!/usr/bin/env python3
"""Checks on the research artifacts, including deliberate negative controls."""
import csv
import json
import sys
from pathlib import Path
import mpmath as mp
from series_probe import HERE, build, direct, reference

truth = json.loads((HERE/'truth.json').read_text())
if len(sys.argv) > 1:
    audit = json.loads(Path(sys.argv[1]).read_text())
    assert audit['limit'] == truth['limit'] and audit['scenes'] == truth['scenes']
    with mp.workdps(70):
        for a, b in zip(truth['rows'], audit['rows'], strict=True):
            assert (a['scene'], a['x'], a['n'], a['escaped']) == (b['scene'], b['x'], b['n'], b['escaped'])
            for key in ['z', 'q']:
                av = mp.mpc(*a[key]); bv = mp.mpc(*b[key])
                assert abs(av-bv) <= mp.mpf('1e-30')*max(abs(av), mp.mpf('1e-100'))
    print('100 vs 150 digit truth audit: 21/21 agree to 30 relative decimal digits')

with mp.workdps(100):
    ref = reference(mp.mpc('0.25'), 300)
    init = build(ref, 1e-6, 4)
    assert init[0] > 200
    # Check initializer state AND parameter derivative at off-axis disk samples.
    for x in [mp.mpc('0.8', '0.1'), mp.mpc('-0.5', '0.5'), mp.mpc('0', '-0.9')]:
        n, _, z, d = direct(mp.mpf('.25')+mp.mpf('1e-6')*x, init[0])
        _, _, zref, _ = direct(mp.mpf('.25'), n)
        b = init[1]
        p = sum(mp.mpc(v)*x**k for k, v in enumerate(b))
        q = sum(k*mp.mpc(b[k])*x**(k-1) for k in range(1, len(b)))
        assert abs(z-zref-p) < mp.mpf('1e-12')
        assert abs(mp.mpf('1e-6')*d-q)/abs(mp.mpf('1e-6')*d) < mp.mpf('1e-8')
    # Diagnose the tip control: adding u to Z loses a real bailout crossing.
    u = complex(-.6, .45)*1e-29; Z = -2+0j
    naive = abs(Z+u)>2
    split = ((abs(Z)**2-4)+2*(Z.conjugate()*u).real+abs(u)**2)>0
    assert not naive and split
    assert direct(mp.mpc(-2)+mp.mpc('-.6', '.45')*mp.mpf('1e-29'), 100)[0] == 1
    print('initializer state/derivative spot checks and split-bailout diagnosis passed')

pixels = list(csv.DictReader((HERE/'results/pixels.csv').open()))
assert len(pixels) == 126
assert sum(r['escaped_truth']=='True' for r in pixels if r['order']=='0') == 19
assert all(r['supported']=='True' for r in pixels)
assert all(r['n']==r['escape_truth'] for r in pixels if r['scene']!='tip-29')
assert any(r['correct']=='False' for r in pixels if r['scene']=='valley-14')
assert all(r['prefix']=='0' for r in pixels if r['scene']=='tip-29')
print('126 records: expected positive and negative controls preserved')
