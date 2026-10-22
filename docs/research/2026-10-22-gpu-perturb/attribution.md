# GPU32 fixed-corpus attribution, 2026-10-22

Checkpoint research only: no production promotion, tolerance edit, speed claim or completed issue. All six cameras, size128x72, ss1, iteration cap10000 and requested nu/de/normal columns are unchanged from GPU32's original corpus. Device AMD Radeon RX5700XT/RADV. Named local checks ran under positive eight-worker leases.

## Actual evidence

- 765 (tree0cc3db7f): diagnostic4096step dispatch, first rebase metadata, termination and existing admission inputs. Raw `/home/juno/fractodactyl/target/gpu32-attribution/states.tsv` (11.8MB); analysis `attribution-4096.json`.
- 766 (tree7bed4f6c): original CPU route, actual escaped-outcome iteration trace, summarized in `cpu-escape-histogram.json`. Tim9216 escaped at6823..8441, mean6849.826280381944. Seahorse9208 escaped35..4535,8Interior; deep seahorse9216 escaped exactly79; exterior9216 escaped exactly7; cusp9216Interior; valley9216Unresolved at10000. Histogram keys are pixel binary exponents: -15 seahorse,-54 deep seahorse,-34 exterior,-274 Tim.
- 767 (tree239c3765): one approved10000step/full-reference diagnostic, unchanged guards. CPU fallback explicitly requests its ORIGINAL default reference (`None`) rather than sharing the experimental Float GPU reference. This deliberate diagnostic-only repeated reference isolates the prior Float-fallback discrepancy, not a proposed production architecture. GPU reference providers remain F64 for F64 views and experimental Float for deeper views. GPU reference lengths80/10001/80/10001/8/6827, never increased beyond requested cap plus endpoint.
- 768 (same tree): actual `fd compare` of766CPU versus767GPU+fallback, all55296 samples,0class mismatches,0nu/de/normal threshold failures. All Tim nu values identical; prior two Float-fallback nu errors disappear. Only exterior has numerical differences:83nu values differ by at most8.882e-16 (5.562e-6px), max relative DE2.2671e-7, normal0. Header bytes differ due opt-in kernel name, not claimed byte identity.

Diagnostic cold control totals: CPU trace7660.590807709s; GPU trace7673.137954109s. These include substantial per-sample trace output and extra diagnostic GPU arithmetic, so they are NOT comparable production-performance measurements. Both builds excluded from reported cold totals. GPU accepted688 exterior escaped samples only; all other views0. No speed or useful deep coverage demonstrated.

## First failed predicate at terminal candidate

Counts follow the existing ordered terminal decisions: status/budget, escape margin, derivative bound, displacement, DE, normal. They are not a claim to the earliest iteration a hypothetical final-output tolerance would cross. First rebase state is separately observed, not assumed causal. No range/nonfinite/invalid terminations occurred. Reference-end rebase is a reset, not itself rejection.

| View |4096steps|10000steps|
|---|---|---|
|seahorse|3347escape margin,298derivative,5560displacement,7DE,4budget|same|
|cusp|9216budget|9216budget; CPU interior, escaped-only route cannot admit|
|deep seahorse|9216displacement|same, n79, no rebases|
|valley|9216budget|9216budget; CPU unresolved, escaped-only route cannot admit|
|exterior|8528displacement,688accepted|same|
|Tim arbitrary save|9216budget|9216escape margin, n6822..6831|

4096-step cusp/valley/Tim rebase at the FINAL allowed iteration only: no later GPU iteration consumes the lossy reset. Therefore the original zero deep coverage cannot be attributed to earlier rebase loss. At10000steps valley first resets at cancellation for4607 samples/reference-end for4609, but CPU remains unresolved. Tim first cancellation rebase occurs for8901samples,315 never rebase; all reject.

Representative Tim pixel(0,0): first cancellation reset n6821,m6821; upload radius7.7313e-8, accumulated e BEFORE reset840670.9375, prior delta magnitude261.902. At terminal n6822,m0, |z|4.71696e9, ez8.22955e11, normalized |d|0.967779, ed220.0119, derivative exponent303. The propagated reference-upload rebase contribution is0.07566, dwarfed by total ez. This disproves reference-end/absolute-upload injection as the dominant error for this sample. The conservative early possible-escape stop correctly prevents corrupt admission. Other representatives and exact camera strings are in JSON.

Deep seahorse pixel(0,0): n79,m79,no reset; |z|3.0920151e11,ez1.1799071, normalized |d|0.619849,ed0.000345446,dexp60. DE≈2.930494e11px. Existing shared encoder's nu-rounding term alone contributes0.007852px; orbit uncertainty contributes0.021145px, both beyond5e-4 admission. A higher-precision GPU mantissa cannot remove the existing f64-output rounding floor; do not weaken that guard just to admit this featureless frame.

## Actionable next proposal, not yet implemented

A bounded next numerical implementation should reuse existing `ff.wgsl` CE/FE/FF primitives (including mandatory runtime `ff_init` reassociation guard), not create another arithmetic copy. Extend the generic GPU state/transport adapter to carry hi+lo complex mantissas with exponent, derive an honest rounding/transport error recurrence for those operations, retain shared CPU encoder/admission budgets and full requested iteration budget. Prove actual Tim and varied-frame coverage against the unchanged baseline, actual negative restart cases and complete cold cost before promotion. No secondary reference groups are justified by this census: Tim's cumulative error already dominates before first reset. Sparse checkpoints or reducing64u to a fitted number would not prove output safety. F64 encoded-nu floor remains a legitimate decline on deep seahorse. A CE implementation may still fail or lose on net cost; no promise of GPU11's4x target.

`attribution-diagnostic.patch` records the diagnostic changes relative to stoppedc3aca58d (final10000variant); apply only in research. `attribution-analysis.py` parses4096raw trace; change its base path for10000 trace. Exact executable source snapshots are retained by Ishoo jobs765–768. Diagnostic source changes were reverted to c3aca58d before stopping; only research evidence added. Original prototype remains unpromoted and retains its known Float-fallback issue until deliberately corrected.
