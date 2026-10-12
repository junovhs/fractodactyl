# Research method: construct, compute, understand, extend

## Primary objective

Use mathematical computation to **find new solutions**. A known technique may supply the crucial building block; the experiment should answer a new consequential question beyond its published result. Do not equate a close citation with a solved application, and do not default to trying to beat that citation in timing alone.

## Default discovery cycle (Track A)

1. Identify a tractable unresolved proposition from [FRONTIER](FRONTIER.md), including promising unanswered parts of older P-cards.
2. For each plausible published ingredient, write the exact known statement, the mathematical assumption it uses, and the *new extension or combination* to derive. A search is an accelerator of insight, not an obligatory series of vetoes.
3. Produce an **explicit falsifiable construction**: equations/recurrence, input domain, proposed error mechanism or new algorithm, expected observable, and a counterexample that would reject it.
4. **Run a cheap probe in Chat** when computation can discriminate it. Prefer a <30-second experiment; state seeds/precision and actual tool output. Start with correctness and pathologies, then costs only if relevant.
5. Use observed failure/success to formulate the next improved construction or a derived obstruction. Report the strongest counterexample; no success claims from merely plausible reasoning.
6. Record the exploration even if negative. Mark `EXPLORATORY`, novelty unverified and any missing reproducibility artifacts. **No three-query dossier or pretest Git commit is needed just to compute an exploratory numerical example.**
7. Escalate only a promising, precisely stated hypothesis to Track B; or keep working an informative unresolved obstacle. A literature-only session is an explicit exception, not the preferred default output.

## Confirmatory experimental protocol (Track B)

Before the *frozen confirmatory* run, carry out external prior-art research and preregister linked P/PA files as described in [PRIOR_ART.md](PRIOR_ART.md). A related published technique may be part of `TRIAGE: EXTEND` or a justified `EVALUATE` question; merely proving a core algorithm old does not invalidate a genuinely new consequence. Preserve previously run explorations separately and do not claim the confirmation was the first test.

- **Accuracy first:** independent higher-accuracy oracle and pathological/domain cases; default maximum absolute state error ≤1e-7 and zero wrong discrete classes unless the specific P-card chooses a defensible different threshold. State and derivative checks if relevant. Empirical tests do not certify all inputs.
- **Strongest meaningful comparison:** when measuring benefits, compare against correct same-math and specialist methods with equal permitted caching, composition, compilation, batching and validation. Do not engineer a weak baseline or assume known algorithm must be beaten if the novelty is mathematical rather than economic.
- **Measured cost:** cold build/preprocessing/setup, warm kernel, realistic reuse count and actual amortized totals, in absolute units. At least three repeated paired/interleaved timings with ranges for a promising signal; report hardware/environment and uncertainty.
- **No universal speed floor:** 1% improvement may matter when robust and widely used. Preserve frozen legacy gates and distinguish a failed historical performance target from a smaller valid effect.
- **Honest provenance:** `REPORTED`, `REPRODUCED`, `CERTIFIED`, and `PROOF: NONE/INTERVAL/LEAN/OTHER` are separate from novelty. Lean is optional, relevant to mathematical identities when warranted.
- **Decision:** test the specific hypothesis, explain negative results, preserve smaller true gains and ask what extension the evidence makes possible; avoid routine benchmark races unless they illuminate that question.

## Record and handoff

All attempted computations must have an R-card or linked explicit numerical record with equations, actual observed values/failures and status. Exploratory records may legitimately be `POD: NO`; this is not a reason to skip the computation. Confirmatory `POD: YES` requires the [10/10 audit](PROOF_OF_DONE.md) and Git read-back. Update [INDEX](INDEX.md) and [MAP](MAP.md) on new P/R IDs; update [FRONTIER](FRONTIER.md) when priorities materially change.

The branch is a documentation-only research tree. Use the Chat compute sandbox for scripts; no renderer/source or binary output is committed without permission. Never touch `main`.
