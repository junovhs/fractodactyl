# Agent investigations — received October 10, 2026

These are preserved source reports from the owner's Fractodactyl research conversation. They were **received on 2026-10-10** and filed beneath the **October 9 research directory at the owner's request**. The source text is unchanged; readable filenames replace generic pasted-text upload names.

## Full agent reports

1. [Automatic dynamical compiler](01-automatic-dynamical-compiler-agent-report.md) — discovering finite-period recurrent structures, generating guarded Koenigs/return-map operators, and evaluating whether compilation pays.
2. [Near-parabolic transit acceleration](02-near-parabolic-transit-agent-report.md) — real-cusp analytic fast-forward with error estimates; complex-domain and BLA comparisons remain open.
3. [Factored deep-ladder return maps](03-factored-ladder-return-maps-agent-report.md) — building high-period operators from a short-cycle factorization, aimed especially at the period-16,116, ~1e-1000 Misiurewicz ladder.

## Agent 1 follow-up and verified implementation

- [Automatic compiler follow-up session](01-automatic-compiler-followup-session.txt) — a **partial pasted session transcript**, including intermediate findings, experiments, and progress entries. It is not a self-contained final report. The raw upload text is preserved without editorial cleanup.
- [Automatic Misiurewicz compiler: native prototype results](01-automatic-compiler-native-prototype-results.md) — snapshot copied without editing from the research-branch implementation report. Includes complete-frame numerical gates, repeat cold-frame timings and the mixed-classification test.

Source implementation: [research/auto-misiurewicz-discovery](https://github.com/junovhs/fractodactyl/tree/research/auto-misiurewicz-discovery).

CI evidence: [repeated cold-frame runs](https://github.com/junovhs/fractodactyl/actions/runs/38033469892) and [mixed-classification frame](https://github.com/junovhs/fractodactyl/actions/runs/38033537764).

## Provenance and limits

These are **research records, not accepted production guarantees**. The first report in this folder predates the native implementation; read the later report for updated results. The near-parabolic test was not measured against the production BLA renderer; the deep-ladder operator factorization had not been implemented end-to-end when reported. Mainline `fd` is not changed by archiving these documents.
