# Research lineage map

Arrows mean **motivated or tested**, not proof of mathematical implication. Reducing a domain or changing a baseline forks a question: it never silently rewrites the original result.

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
  P030 --> P080["P-080 · q4 guarded PWM"]\n  P080 --> R006["R-006 · 2/128 failed gate"]\n  R006 --> P081["P-081 · q5 same model"]\n  P081 --> R007["R-007 · >=3x gate failed; warm signal"]
  R007 --> P082["P-082 · amortized-use test"]\n  P030 --> P070["P-070 · Multistate event-domain generalization"]
  classDef fail fill:#ffefef,stroke:#b94a48,color:#74211f
  classDef maybe fill:#fff7e5,stroke:#b98a35,color:#604615
  classDef untested fill:#f2f5f9,stroke:#8090a0,color:#253449
  class R002,R004,R005,R006,R007 fail
  class R001,R003 maybe
  class P030,P040,P051,P060,P070,P082 untested
```

## Clickable reading order

[P-000](PROMPTS/P-000.md) → [P-010](PROMPTS/P-010.md) → [R-001](RESULTS/R-001.md) → [P-020](PROMPTS/P-020.md) → [R-002](RESULTS/R-002.md) → [P-021](PROMPTS/P-021.md) → [R-003](RESULTS/R-003.md) → [P-022](PROMPTS/P-022.md) → [R-004](RESULTS/R-004.md) → [P-030](PROMPTS/P-030.md).

Parallel track: [P-050](PROMPTS/P-050.md) → [R-005](RESULTS/R-005.md) → [P-051](PROMPTS/P-051.md).

Questions branching from the composition result: [P-040](PROMPTS/P-040.md), [P-060](PROMPTS/P-060.md). Measured negative branch: [P-080](PROMPTS/P-080.md) → [R-006](RESULTS/R-006.md) → [P-081](PROMPTS/P-081.md) → [R-007](RESULTS/R-007.md) (accuracy repaired, >=3x cold gate failed) → [P-082](PROMPTS/P-082.md) (new amortized workload). Future multi-state extension: [P-070](PROMPTS/P-070.md).

[All statuses and one-line decisions](INDEX.md).
