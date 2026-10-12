# Research method

## Experiment contract

1. **Name the problem.** Measurable payoff, realistic task distribution, and transferable dynamical mechanism.
2. **Register the hypothesis.** Create a 150-word-or-fewer P-card stating mechanism, baseline, tiny probe, numeric accuracy criterion, kill criterion, and next decision.
3. **Compete fairly.** The rival can cache, reuse, precompute, compose maps and use specialized methods, exactly as our candidate can. Measure **cold** time and warm time separately.
4. **Probe fast.** Target under 30 seconds in Python, then under five minutes for promotion; no long experiments without a first-gate win.
5. **Correctness first.** State error must be measured against an independently accurate reference. Empirical holdout errors are not error certificates. For switched systems compare **event sequence and branch**, parameter derivatives, and state.
6. **Reproduce.** Log exact equations, parameters, seeds, sample counts, hardware, numerical precision, packages/versions, compiler warm-up, timing ranges, costs, precision failures and provenance of executable/raw data. Missing items must be marked missing.
7. **Decision.** KEEP for a valid, competitive, well-scoped result; KILL on the registered gate; CONDITIONAL for a partial win; SUPERSEDED if a better rival overturns it; UNTESTED for questions without experiments.
8. **Fork, don't erase.** Create follow-up P-cards linked from the previous R-card, update MAP and INDEX. Retain dead ends and corrections.

## Default gates (unless overridden in a P-card)

- Error: max absolute state error ≤ 1e-7 on the stated holdout; **zero wrong switching branches or event itineraries**.
- Speed: at least 3× **cold** time reduction against the **fastest accurate same-math rival** at the specified workload.
- Error certification: mark only VERIFIED-EMPIRICAL until a validated numerical enclosure proves a result. No pixel displacement claims from state error alone.
- Automatic operator admission: may decline. Domain selection cost and failures count in cold time.

## Provenance tags

**REPORTED** = measurements stated in a prior chat, not independently recreated from frozen files here. **REPRODUCED** = executable, command, seed, and raw results archived and rerun. **CERTIFIED** = proved enclosure/error contract, with proof attached. All current R-cards are REPORTED.

## Scope safety

This branch should contain documentation and table data only. It is **not for merging** into Fractodactyl main because its tree intentionally omits the renderer. More complex experiments belong in the sandbox and can be summarized here.
