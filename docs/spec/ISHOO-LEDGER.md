# Ishoo Ledger Reconciliation

> **Historical (2026-10-06 bootstrap).** This was the one-time list used to seed Ishoo after the reset. It is not the live ledger: issues, plans and ADRs live in the Ishoo store (`ishoo_status`, `ishoo_list`), and the current order is in [STATE.md](STATE.md). Several items below are done, superseded or declined.

A local agent with Ishoo access should reconcile the project ledger before large implementation work.

## New active plan

Suggested name: **Genuine Deep Atlas v0**

Goal:

> Demonstrate that a path-specific 2–3 GB mathematical atlas can materially reduce warm genuine Mandelbrot video rendering cost through reusable local coordinates, references, operators, and screen-space refinement, without fake-depth substitution.

Preserve completed historical work. Supersede incompatible twin-swap/studio backlog rather than deleting history.

## Decisions to file

### ADR-A01 — Retire twin substitution as core architecture
Accepted. Old PoCs remain evidence; the default product goal is genuine depth.

### ADR-A02 — Optimize warm path rendering via offline compilation
Accepted. Cold arbitrary frames may remain expensive.

### ADR-A03 — Bounded path-specific atlas
Accepted. 2.5 GiB target, 3 GiB default hard ceiling.

### ADR-A04 — Canonical payloads are mathematical, not raster imagery
Accepted.

### ADR-A05 — Screen-space error/deadline controls refinement
Accepted principle; thresholds experimental.

### ADR-A06 — Content-addressed immutable semantic chunks
Accepted.

### ADR-A07 — Separate core fractal computation from appearance/shading
Accepted.

### ADR-A08 — Exact hierarchical addressing and local coordinates
Accepted at principle level; schema may evolve.

### ADR-A09 — Rust production engine; Python oracle/research only
Accepted unless explicitly revisited.

### ADR-A10 — Approximation layers require explicit validity/error/fallback contracts
Accepted.

### ADR-H01 — 128×128 outer tiles + 8×8/16×16 microblocks
Proposed experiment, not yet accepted.

### ADR-H02 — Offline ball/interval certification + cheap GPU consumption
Proposed experiment.

### ADR-H03 — Fixed semantic orbit slabs for CAS
Proposed experiment.

## Initial issue backlog

### RESET-01 — Reconcile Ishoo after architecture reset [P0]
Create the plan/ADRs/issues here, supersede incompatible old backlog, and ensure `ishoo_brief` describes the atlas direction.

### BASE-01 — Define palette-independent renderer outputs [P0]
Versioned semantics for classification, smooth value/potential, derivative/distance/normal data, uncertainty.

### BASE-02 — Genuine scalar baseline renderer [P0]
Deterministic Rust renderer for known locations; checked against higher-precision oracle.

### BASE-03 — Benchmark/correctness harness [P0]
Machine-readable timing, memory, iterations, bytes, fallback, and error.

### ADDR-01 — Exact dyadic tile address model [P0]
Deterministic parent/child/sample mapping, including extreme logical depths.

### ATLAS-01 — Content-addressed chunk store [P0]
Canonical serialization, integrity verification, dedup.

### ATLAS-02 — Tile and frame manifest schema [P0]
Thousands of frame manifests share math chunk hashes without duplication.

### ATLAS-03 — Atlas byte-budget enforcement [P0]
2.5 GiB target / 3 GiB hard cap, with byte accounting and planning behavior.

### TILE-01 — Outer tile benchmark [P1]
Compare 64/128/256/(512) for metadata, overfetch, reuse, divergence, cache behavior.

### GPU-01 — Microblock active-queue renderer [P1]
Compare 8×8 and 16×16. Finished/easy blocks stop consuming expensive work.

### LOD-01 — Fractal screen-space error contract [P0/P1]
Define the conservative refine-vs-accept rule independently of final color.

### LOD-02 — Progressive refinement phases [P1]
Parent preview → sparse/subpixel phases → compact unresolved blocks → final/fallback.

### LOD-03 — 1–3 px hidden-detail / 4 px readiness experiment [P1 research]
Test the user's screen-space intuition directly and use evidence to set policy.

### REF-01 — Reference orbit slab format [P1]
Prototype semantic iteration slabs and compact formats.

### REF-02 — Multi-frame reference reuse [P1]
Demonstrate a reference serving multiple tiles/frames and measure savings.

### REF-03 — Path-aware reference placement optimizer [P2]
Beat naive one-reference-per-frame/tile on bytes or work.

### ACC-01 — Reusable BLA operator payload [P1]
Established acceleration represented as an atlas chunk with validity tests.

### ACC-02 — Certified BLA remainder bounds [P2]
Attach conservative error bounds usable by LOD/fallback.

### ACC-03 — Higher-order operator spike [P2]
Compare first-order vs second/higher maps on bytes, build time, valid domain, fallback, speed.

### RET-01 — High-period return-map codec spike [P1 research, high risk]
Compare raw perturbation, BLA, and a higher-order local return map as period increases. Measure compile time, bytes, online evaluations, error, fallback, and frame time.

### RET-02 — Return-map branching/validity model [P2]
Only if RET-01 is promising.

### CERT-01 — Offline ball/interval certificate spike [P2]
Measure whether compact proofs/bounds justify their cost.

### SCHED-01 — Known-path footprint planner [P1]
Camera path → future tile/microblock demand and first-use frames.

### SCHED-02 — Build-cost-aware deadline scheduler [P1]
Schedule reference/operator/certificate generation by first-use deadline and remaining cost.

### SAMPLE-01 — Exact sample cache [P2]
Cache results keyed by exact mathematical sample coordinate.

### SAMPLE-02 — Nested/dyadic sampling lattice spike [P2 research]
Deliberately increase exact sample reuse between zoomed frames; measure reuse vs quality.

### SHADE-01 — Cheap late shading pipeline [P1]
Multiple looks from one mathematical result cache without deep recompute.

### VIDEO-01 — 20–30 second Atlas v0 compiler [P0 milestone]
Known genuine path compiled within byte budget.

### VIDEO-02 — Atlas v0 renderer/player [P0 milestone]
Render VIDEO-01 primarily from atlas state.

### VIDEO-03 — Independent-frame control renderer [P0 milestone]
Same frames without movie-wide reuse.

### BENCH-01 — Atlas vs independent benchmark [P0 milestone]
Compare build time, warm/cold frame time, bytes, reuse, fallback, peak memory, correctness.

### BENCH-02 — Depth-scaling benchmark families [P1/P2]
Generic expanding, parabolic cusp, high-period mini, period-doubling/renormalization, awkward boundary, curated long path.

### BENCH-03 — Warm-cost depth slope [P1]
Measure whether warm cost systematically rises with depth; target is slope approaching zero across an increasing useful range.

### INTERACTIVE-01 — Interactive clipmap mode [future]
Blocked by video proof. Reuse the same atlas payloads with ancestor preview and predictive prefetch.

## Preferred execution order

RESET-01 → BASE-01/02/03 → ADDR-01 → ATLAS-01/02/03 → SCHED-01 → REF-01/02 → ACC-01 → LOD-01/02 → SCHED-02 → SHADE-01 → VIDEO-01/02/03 → BENCH-01 → RET-01 → BENCH-02/03.

Do not spend significant time on UI before the Gate C reuse hypothesis is proven.
