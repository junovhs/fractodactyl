# GPU-32 shared Float reference checkpoint (not promoted)

The default `Fixed` builder and its escape²≤2³² integer-limb invariant stay intact.
The experimental consumer uses existing astro-float-backed `Complex::sqr/add`
with one shared collection driver, one exact-decimal reference for GPU transfer
and CPU restart. `Float` operations round nearest-even at the wider operand's
precision; no absolute camera coordinate is converted to f64 for orbit arithmetic.

Domain: declared fraction accuracy b≥128, working mantissa P≥b+128 bits,
2≤escape radius R≤2⁴⁰, rounded centre L1≤4 (so true L1<8). The f64 stopping test
implies |Float Z|≤2R before every continued step; f64 conversion error cannot
approach a factor of two in this normal magnitude range. All scalar intermediates
of Complex::sqr/add are <2⁸⁵. Adding its scalar RN defects, propagation inside
that one arithmetic stage, and decimal parse error is conservatively <2^(96−P),
therefore <2^(−b−32). This is much smaller than the existing `error_radius`
absolute local-defect allowance 2^(4−b). The existing recurrence separately
propagates accumulated error and encloses storage rounding into f64. Unsupported
precision/centre/radius inputs decline; independent oracle agreement alone is
not the justification for this bound.

Check705 compares every point with independent mpmath at b+768 bits. Recorded cases
(seahorse, valley, large escape and zero and one since-removed camera) total26,916
points; all radii finite, worst actual-error/radius0.49524. Domain rejection also
passes. This supports the derivation but does not establish a useful GPU route.

Actual unchanged six-view CLI corpus703 still admits zero representative deep
GPU samples (688 exterior samples only). It also has two nu-displacement failures
on seahorse1e−14, maximum0.0014433px, entirely in CPU fallback using the changed
reference; classes, DE and normals pass. This route is NOT ready to land. No additional tuning or promotion was attempted.
