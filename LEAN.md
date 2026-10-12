# Formal verification: when Lean is useful

**Lean is an optional high-assurance escalation, not a prerequisite for exploratory probes.**

## What to prove

For a successful reusable operator, clearly state a **bounded theorem**: hypotheses on state/parameter domain, convergence or non-singularity, exact identity or finite truncation remainder, and what output/derivative/itinerary conclusions follow. Separate the real/complex mathematical theorem from actual finite-precision execution. For rounding/implementation claims, use verified interval/ball enclosures or a formally connected implementation specification; a Lean proof of an abstract identity alone is insufficient.

If a cheap independent numerical enclosure already certifies the required bound, prefer it over formalizing a large library before knowing the idea pays.

## Escalation

1. **Probe:** empirical independent numerical oracle; kill early on an accuracy failure. No Lean requirement.
2. **Promote:** rigorous interval/ball/Taylor-model error bounds on practical domains; independently repeated performance tests against specialist rivals.
3. **Formalize only if justified:** small, reusable lemma, nontrivial delicate proof, or safety-critical certification; include the exact Lean source commit and type-check log. Audit for `sorry`, `admit`, unsafe axioms and any unsupported assumptions.
4. **Claim scope:** attach the theorem, verified toolchain, all assumptions, and connection to the numerical output. A checked lemma does **not** prove the performance benchmark or the whole simulator correct.

## Execution availability

GitHub Actions can check an actual Lean/Lake project using [`leanprover/lean-action@v1`](https://github.com/leanprover/lean-action). Such a project requires a pinned Lean toolchain and appropriately configured sources/dependencies, and typically a `lake build` step. **No workflow or Lean package has been created in this notebook.** For serious proofs, create a separately authorized proof workspace/repository rather than stuffing source files into this documentation-only branch.

A general ChatGPT compute sandbox may not have `lean`, `lake`, or `elan` installed; check before claiming local verification. A written Lean snippet that was not run is **unverified**, not a proof. An agent with installation/build access may use Lean when warranted and should record the successful checker output.

## Proof-of-done relation

Report `PROOF: NONE` for ordinary experimental results. Mark `PROOF: LEAN` only when the relevant theorem was actually type-checked and its source + dependencies are retained; `PROOF: INTERVAL` only when the bound construction and trusted interval calculation are documented. See [PROOF_OF_DONE.md](PROOF_OF_DONE.md).
