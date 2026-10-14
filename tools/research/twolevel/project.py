#!/usr/bin/env python3
"""RDNA1 (RX 5700 XT) operation-count projection for the two-level float32 scheme.

Issue slots per lane per iteration (FMA = 1 slot; binary64 = 16 slots on Navi 10):
  F32_ITER  float32-with-exponent pixel step off the tile orbit, with variance guard  ~43
  F64_ITER  binary64-with-exponent one-level perturbation step (fd scaled tier style)  ~28 FP64 + 8 int
Tile reference: one binary64 step per tile per raw iteration (separate kernel, one lane per
tile), plus 24 B/iteration/tile written and read once per workgroup.
Fallback pixels: rerun from scratch in binary64 (compacted queue, no divergence assumed).
These are projections from operation counts, not GPU measurements.
"""
import sys, numpy as np
import analyze as A

F32_ITER = 43
F64_ITER = 28 * 16 + 8
LANES, CLK = 2560, 1.755e9          # RX 5700 XT, game clock
BW = 448e9                          # bytes/s
NPX = 1920 * 1080

def project(rec, T, rule, tileref_per_px):
    r = A.load(rec)
    tn = r['tn']; nmax = 100000
    f64_steps = np.where(tn >= 0, tn, nmax)           # binary64 per-pixel cost = full iteration count
    fb = rule(r)
    f32_steps = r['steps']
    slots_f32 = F32_ITER * f32_steps.sum() + F64_ITER * f64_steps[fb].sum() + F64_ITER * tileref_per_px * len(tn)
    slots_f64 = F64_ITER * f64_steps.sum()
    mem_s = 2 * 24 * tileref_per_px * NPX / BW       # write + read of tile orbits for a full frame
    t32 = slots_f32 / len(tn) * NPX / (LANES * CLK) + mem_s
    t64 = slots_f64 / len(tn) * NPX / (LANES * CLK)
    wfb = F64_ITER * f64_steps[fb].sum() / slots_f32
    return fb.mean(), wfb, t32 * 1e3, t64 * 1e3, t64 / t32

def rule_nu(r, lam=4, tol=1e-3, Fde=1.0, Fz=1.0):
    # escaping pixels: lambda*sigma <= tol and <= Fde*DE (linear regime);
    # non-escaping pixels: per-step lambda*sigma_n <= Fz*|z_n| (no unflagged orbit perturbation)
    esc = r['rn'] >= 0; ls = lam * r['sig']
    return (esc & ~(ls <= tol)) | (esc & ~(ls <= Fde * r['rde'])) | (~esc & (r['zrat'] < (lam / Fz) ** 2))

def rule_full(r, lam=4, tol=1e-3, Fde=1.0):
    esc = r['rn'] >= 0; ls = lam * r['sig']
    return rule_nu(r, lam, tol, Fde) | (esc & (r['rde'] > 1e-3) & ~(ls <= 2e-3 * r['rde']))

if __name__ == '__main__':
    # args: REC T TILEREF_ITERS_PER_PX
    rec, T, tr = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
    for name, rule in (('nu', rule_nu), ('full', rule_full)):
        f, wfb, t32, t64, s = project(rec, T, rule, tr)
        fb = rule(A.load(rec)); rr = A.load(rec)
        both = (rr['tn'] >= 0) & (rr['rn'] >= 0)
        d = np.where(both, np.abs(rr['rnu'] - rr['tnu']) * A.LN2 * rr['tde'], 0)
        kept = ~fb
        cls = int((((rr['tn'] >= 0) != (rr['rn'] >= 0)) & kept).sum())
        relde = np.where(both & (rr['tde'] > 1e-3), np.abs(rr['rde'] / np.where(rr['tde'] > 0, rr['tde'], 1) - 1), 0)
        print(f"{name:4s} kept: max {(d*kept).max():.2e} px, over1e-3 {int(((d>1e-3)&kept).sum())}, cls {cls}, DE>0.2% {int(((relde>2e-3)&kept).sum())} | interior-fallback {100*(fb & (rr['rn']<0)).mean():.3f}%")
        print(f"{name:4s} fallback {100*f:.3f}% (cost share {100*wfb:.1f}%) | proj f32 {t32:.1f} ms vs f64 {t64:.1f} ms -> {s:.2f}x")
