# Research method

## Scope of agent autonomy

The durable mission is in [GOAL.md](GOAL.md); the provisional highest-information leads are in [FRONTIER.md](FRONTIER.md). There is no permanently prescribed problem: agents select among current or new cheap experiments based on actual evidence and anticipated impact, documenting decisions. Findings, including failed and overturned results, are the product.

**Every new R-card must finish with the nine-step [binary proof of done](PROOF_OF_DONE.md).** A failed hypothesis may be `POD: YES`; a favorable but poorly evidenced result is `POD: NO`. The gate and evidence classifications are separate. [Lean formalization](LEAN.md) is optional and selective, never a default 30-second probe requirement.

## Experiment contract

1. **Name the problem.** Measurable payoff, realistic task distribution, and transferable dynamical mechanism.
2. **Register the hypothesis.** Create a 150-word-or-fewer P-card stating mechanism, baseline, tiny probe, numeric accuracy criterion, kill criterion, and next decision.
3. **Compete fairly.** The rival can cache, reuse, precompute, compose maps and use specialized methods, exactly as our candidate can. Measure **cold** time and warm time separately.
4. **Probe fast.** Target under 30 seconds in Python, then under five minutes for promotion; no long experiments without a first-gate win.
5. **Correctness first.** State error must be measured against an independently accurate reference. Empirical holdout errors are not error certificates. For switched systems compare **event sequence and branch**, parameter derivatives, and state.
6. **Reproduce.** Log exact equations, parameters, seeds, sample counts, hardware, numerical precision, packages/versions, compiler warm-up, timing ranges, costs, precision failures and provenance of executable/raw data. Missing items must be marked missing.
7. **Decision.** Keep the original preregistered numerical GATE immutable. Separately record current research opportunity: KEEP for a trustworthy end-to-end improvement on a relevant workload; CONDITIONAL for a useful partial/warm-only signal with a cheap next falsifier; KILL when the tested claim fails or further investigation has poor economics; SUPERSEDED if a stronger rival overturns it. Never relabel an older frozen 3x failure as a PASS.
8. **Fork, don't erase.** Create follow-up P-cards linked from the previous R-card, update MAP and INDEX. Retain dead ends and corrections.

## Default gates (unless overridden in a P-card)

- Error: max absolute state error ≤ 1e-7 on the stated holdout; **zero wrong switching branches or event itineraries**.
- Speed: **no universal >=3x hurdle**. Predeclare the relevant lifecycle and minimum useful improvement. For incremental optimization, test **>=1.10x end-to-end** versus the fastest accurate same-math/specialist rival when the timing effect is robust to noise; smaller gains may justify pursuit if absolute compute savings are material. Keep >=3x for major breakthroughs and any old P-card that explicitly froze it.
- Error certification: empirical agreement alone is not a certificate; report `PROOF: NONE` until an auditable enclosure or a formal checked theorem proves the relevant claim. No pixel displacement claims from state error alone.
- Automatic operator admission: may decline. Domain selection cost and failures count in cold time.
- **Economic accounting:** distinguish one-shot cold, warm kernel, and **realistically amortized total including build/JIT**, with explicit repeat counts, absolute saved seconds, error/coverage and implementation cost. A warm speedup is not by itself an application win. For multiple changes use a combined benchmark; separate-stage speedups follow Amdahl's law and do not automatically multiply.
- **Distraction control:** record every reliable improvement, but only run the next small probe when its expected information and deployment savings outweigh research time. Before investing more, demand representative workloads, strong specialist opponents and noise-resolvable timing.

## Provenance tags

**REPORTED** = measurements stated in a prior chat, not independently recreated from frozen files here. **REPRODUCED** = executable, command, seed, and raw results archived and rerun. **CERTIFIED** = proved enclosure/error contract, with proof attached. All current R-cards are REPORTED.

## Scope safety

This branch should contain documentation and table data only. It is **not for merging** into Fractodactyl main because its tree intentionally omits the renderer. More complex experiments belong in the sandbox and can be summarized here.
