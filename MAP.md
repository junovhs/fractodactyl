# Research lineage map

Arrows mean **motivated or tested**, not proof of mathematical implication. A failed *preregistered gate* is not a verdict that all measured gains are worthless: R-007 failed its 3× single-cold target but R-008 retained a separately tested 1.209× amortized improvement. New workload/competitor/gates require linked questions, never silent rewriting.

```mermaid
flowchart TD
  P000["P-000 · Transfer dynamical shortcuts"] --> P010["P-010 · Fit one-period map"]
  P010 --> R001["R-001 · Faster warm / loses cold"]
  R001 --> P020["P-020 · Koenigs periodic jump"]
  P020 --> R002["R-002 · Broad domain FAILED"]
  R002 --> P021["P-021 · Shrink chart, scale batch"]
  P021 --> R003["R-003 · Local conditional win"]
  R003 --> P022["P-022 · Same-math polynomial rival"]
  P022 --> R004["R-004 · Koenigs outperformed"]
  R004 --> P030["P-030 · Guard switching maps"]
  R004 --> P040["P-040 · Automatic operator chooser"]
  R004 --> P060["P-060 · Repelling-cycle rival in Mandelbrot"]
  P000 --> P050["P-050 · Whole-orbit Newton scan"]
  P050 --> R005["R-005 · Global scan FAILED"]
  R005 --> P051["P-051 · Certified segmented solve"]
  P051 --> R010["R-010 · Exploratory chart seed and handoff"]
  P030 --> P080["P-080 · q4 guarded PWM"]
  P080 --> R006["R-006 · 2/128 failed gate"]
  R006 --> P081["P-081 · q5 same model"]
  P081 --> R007["R-007 · >=3x gate failed; warm signal"]
  R007 --> P082["P-082 · amortized-use test"]
  P082 --> R008["R-008 · 1.209x cold; toy pass"]
  P030 --> P070["P-070 · Multistate event-domain generalization"]
  P030 --> P083["P-083 · Event-time quadratic predictor"]
  P083 --> R009["R-009 · Known-method measurement; pivot"]
  classDef fail fill:#ffefef,stroke:#b94a48,color:#74211f
  classDef maybe fill:#fff7e5,stroke:#b98a35,color:#604615
  classDef untested fill:#f2f5f9,stroke:#8090a0,color:#253449
  class R002,R004,R005,R006 fail
  class R001,R003,R007,R008,R009,R010,P083 maybe
  class P030,P040,P051,P060,P070 untested
```

## Clickable reading order

[P-000](PROMPTS/P-000.md) → [P-010](PROMPTS/P-010.md) → [R-001](RESULTS/R-001.md) → [P-020](PROMPTS/P-020.md) → [R-002](RESULTS/R-002.md) → [P-021](PROMPTS/P-021.md) → [R-003](RESULTS/R-003.md) → [P-022](PROMPTS/P-022.md) → [R-004](RESULTS/R-004.md) → [P-030](PROMPTS/P-030.md).

Parallel track: [P-050](PROMPTS/P-050.md) → [R-005](RESULTS/R-005.md) → [P-051](PROMPTS/P-051.md).

Questions branching from the composition result: [P-040](PROMPTS/P-040.md), [P-060](PROMPTS/P-060.md). Measured negative branch: [P-080](PROMPTS/P-080.md) → [R-006](RESULTS/R-006.md) → [P-081](PROMPTS/P-081.md) → [R-007](RESULTS/R-007.md) (accuracy repaired; old ≥3× gate failed, 2.027× warm signal retained) → [P-082](PROMPTS/P-082.md) → [R-008](RESULTS/R-008.md) (1.209x cold synthetic workload gain). Future multi-state extension: [P-070](PROMPTS/P-070.md).

New pretest branch: [P-030](PROMPTS/P-030.md) → [P-083](PROMPTS/P-083.md) ([PA-P-083](PRIOR_ART/PA-P-083.md)) → [R-009](RESULTS/R-009.md) (historical warm-only gain; **known-method research CLOSED/PIVOT**, no default tuning child). Search-only future prior-art matches go straight to a documented [frontier](FRONTIER.md) pivot, without a forced P/R edge.

[All statuses and one-line decisions](INDEX.md).

Executed exploratory child: [P-051](PROMPTS/P-051.md) → [R-010](RESULTS/R-010.md), conditional exact-initial-condition chart seeding, nonsequential defect budget, and failed out-of-range chart evaluation. Not a certified time-parallel solver or speedup.
