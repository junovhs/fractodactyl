#!/usr/bin/env python3
"""Sweep guard thresholds over per-pixel records written by tile32 (REC=...)."""
import sys, numpy as np
LN2 = np.log(2)
def load(p):
    a = np.fromfile(p, dtype=np.float64).reshape(-1, 10)
    tn, tnu, tde, rn, rnu, rde, sig, zrat, steps, bound = a.T
    return dict(tn=tn, tnu=tnu, tde=tde, rn=rn, rnu=rnu, rde=rde, sig=sig, zrat=zrat, steps=steps)
def score(r, lam=4, Fz=None, Fde=None, tol=1e-3, full=False):
    tn, rn = r['tn'], r['rn']
    esc_t, esc_r = tn >= 0, rn >= 0
    ls = lam * r['sig']
    fb = np.zeros(len(tn), bool)
    fb |= esc_r & ~(ls <= tol)
    if Fz is not None: fb |= r['zrat'] < (lam / Fz) ** 2
    if Fde is not None: fb |= esc_r & ~(ls <= Fde * r['rde'])
    if full: fb |= esc_r & (r['rde'] > 1e-3) & ~(ls <= 2e-3 * r['rde'])
    both = esc_t & esc_r
    d = np.where(both, np.abs(r['rnu'] - r['tnu']) * LN2 * r['tde'], 0)
    cls = esc_t != esc_r
    kept = ~fb
    dekeep = kept & both & (r['tde'] > 1e-3) & (np.abs(r['rde'] / np.where(r['tde'] > 0, r['tde'], 1) - 1) > 2e-3)
    return fb.mean(), (d * kept).max(), int(((d > 1e-3) & kept).sum()), int((cls & kept).sum()), int(dekeep.sum())
if __name__ == '__main__':
    for p in sys.argv[1:]:
        r = load(p)
        print(p.rsplit('/', 1)[-1], 'px', len(r['tn']))
        for full in (False, True):
            for rule in [('none', None, None), ('z0.1', 0.1, None), ('z0.3', 0.3, None), ('z1', 1, None), ('z3', 3, None),
                         ('de0.1', None, 0.1), ('de0.3', None, 0.3), ('de1', None, 1.0)]:
                f, mx, ov, cl, db = score(r, Fz=rule[1], Fde=rule[2], full=full)
                print(f"  {'full' if full else 'nu  '} {rule[0]:6s} fallback {100*f:7.4f}% kept max {mx:.2e} over {ov} cls {cl} DEbad {db}")

def load_alt(p):
    a = np.fromfile(p, dtype=np.float64).reshape(-1, 13)
    r = dict(zip(['tn','tnu','tde','rn','rnu','rde','sig','zrat','steps','bound','an','anu','ade'], a.T))
    return r

def score_xcheck(r, lam=4, tol=1e-3, A=1e-4, Ade=5e-4, full=False, Anu=1e-2):
    """Rounding-diversity cross-check: accept the float32 result if a second float32 run on a
    different tile reference agrees (same class; nu within A px; DE within Ade relative)."""
    tn, rn, an = r['tn'], r['rn'], r['an']
    esc_t, esc_r, esc_a = tn >= 0, rn >= 0, an >= 0
    fb = esc_r & ~(lam * r['sig'] <= tol)
    fb |= esc_r != esc_a
    both_ra = esc_r & esc_a
    dra = np.where(both_ra, np.abs(r['rnu'] - r['anu']) * LN2 * np.maximum(r['rde'], r['ade']), 0)
    fb |= dra > A
    fb |= both_ra & (np.abs(r['rnu'] - r['anu']) > Anu)
    if full:
        rel = np.where(both_ra & (r['rde'] > 1e-3), np.abs(r['rde'] / np.where(r['ade'] > 0, r['ade'], 1) - 1), 0)
        fb |= rel > Ade
    both = esc_t & esc_r
    d = np.where(both, np.abs(r['rnu'] - r['tnu']) * LN2 * r['tde'], 0)
    cls = esc_t != esc_r
    kept = ~fb
    dekeep = kept & both & (r['tde'] > 1e-3) & (np.abs(r['rde'] / np.where(r['tde'] > 0, r['tde'], 1) - 1) > 2e-3)
    return fb.mean(), (d * kept).max(), int(((d > 1e-3) & kept).sum()), int((cls & kept).sum()), int(dekeep.sum())
