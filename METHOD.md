# Research method

## Scope of agent autonomy

The durable mission is in [GOAL.md](GOAL.md); the provisional highest-information leads are in [FRONTIER.md](FRONTIER.md). There is no permanently prescribed problem: agents select among current or new cheap experiments based on actual evidence and anticipated impact, documenting decisions. Findings, including failed and overturned results, are the product.

**Every new R-card must finish with the nine-step [binary proof of done](PROOF_OF_DONE.md).** A failed hypothesis may be `POD: YES`; a favorable but poorly evidenced result is `POD: NO`. The gate and evidence classifications are separate. [Lean formalization](LEAN.md) is optional and selective, never a default 30-second probe requirement.

## Experiment contract

1. **Name the problem.** Measurable payoff, realistic task distribution, and transferable dynamical mechanism.
2. **Register the hypothesis.** Create a 150-word-or-fewer P-card stating mechanism, baseline, tiny probe, numeric accuracy criterion, kill criterion, and next decision.
3. **Compete fairly.** The rival can cache, reuse, precompute, compose maps and use specialized methods, exactly as our candidate can. Measure **cold** time and warm time separately.
4. **Probe fast.** Target under 30 seconds in Python, then under five minutes for discriminating follow-ups; invest more only when accuracy and plausible absolute savings justify it. A small measured gain can warrant a follow-up even if a historical performance threshold failed.
5. **Correctness first.** State error must be measured against an independently accurate reference. Empirical holdout errors are not error certificates. For switched systems compare **event sequence and branch**, parameter derivatives, and state.
6. **Reproduce.** Log exact equations, parameters, seeds, sample counts, hardware, numerical precision, packages/versions, compiler warm-up, timing ranges, costs, precision failures and provenance of executable/raw data. Missing items must be marked missing.
7. **Decision.** Keep the original preregistered numerical GATE immutable. Separately record **the measured effect** and **whether follow-up is worthwhile**: KEEP for a credible, correctly measured relevant end-to-end saving of any magnitude; CONDITIONAL for warm/subsystem gains or unproven transfer; STOP when expected further payoff does not justify the cost; KILL only the *specific falsified hypothesis or gate*; SUPERSEDED if a stronger rival reverses the result. A frozen 3× failure cannot become PASS, but can contain a valid smaller improvement.
8. **Fork, don't erase.** Create follow-up P-cards linked from the previous R-card, update MAP and INDEX. Retain dead ends and corrections.

## Default gates (unless overridden in a P-card)

- Error: max absolute state error ≤ 1e-7 on the stated holdout; **zero wrong switching branches or event itineraries**.
- Speed: **no global minimum**—not 3× and not 1.10×. If a P-card needs a numerical economic gate, predeclare one justified by workload, expected absolute saved time, precision/noise and implementation cost; **1.01×, 1.05× or 1.10×** can all matter when robust. Threefold is only an aspirational breakthrough tier or a **historically frozen** experiment target. A failure to clear the *chosen* gate cannot erase a smaller real observed gain.
- Error certification: empirical agreement alone is not a certificate; report `PROOF: NONE` until an auditable enclosure or a formal checked theorem proves the relevant claim. No pixel displacement claims from state error alone.
- Automatic operator admission: may decline. Domain selection cost and failures count in cold time.
- **Economic accounting:** distinguish one-shot cold, warm kernel, and **realistically amortized total including build/JIT**, with explicit repeat counts, absolute saved seconds, error/coverage and implementation cost. A warm speedup is not by itself an application win. For multiple changes use a combined benchmark; separate-stage speedups follow Amdahl's law and do not automatically multiply.
- **Distraction control:** record every legitimate positive effect, even if not worth further optimization. Use expected saved computation, applicability and experiment cost to ration effort; do not use a fixed speedup multiple as a proxy. A measured 1% gain must be distinguished from timer noise; prioritize representative workloads, strong rivals and controlled repeatability.

## Provenance tags

**REPORTED** = measurements stated in a prior chat, not independently recreated from frozen files here. **REPRODUCED** = executable, command, seed, and raw results archived and rerun. **CERTIFIED** = proved enclosure/error contract, with proof attached. All current R-cards are REPORTED.

## Scope safety

This branch should contain documentation and table data only. It is **not for merging** into Fractodactyl main because its tree intentionally omits the renderer. More complex experiments belong in the sandbox and can be summarized here.
