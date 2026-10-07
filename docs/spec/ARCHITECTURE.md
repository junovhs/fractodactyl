# Architecture Notes and Research Gates

## Mental model

Do not precompute the infinite fractal. **Virtualize it.**

Compile only the path corridor the camera can see, only to the detail the screen can reveal, and store the mathematical work that many future samples can share.

## Screen-space refinement

The core idea is that a feature can contain unlimited hidden structure while occupying only a few output pixels. Fractadactyl should know when the current representation is still sufficient for the requested raster.

Suggested tile states:

- `CERTIFIED_UNIFORM`: no refinement needed for current output semantics.
- `CERTIFIED_APPROXIMATE`: approximation has an explicit bound; accept if its projected error is below threshold.
- `UNRESOLVED_BOUNDARY`: subdivide, supersample, raise precision, or fall back.

The implemented rule (evidence gates, `E_px`, 0.25 px default, `fd lod`) is docs/spec/LOD.md.

For every future chunk, determine the first frame where its current bound would exceed the allowed screen-space threshold. That is its **detail deadline**.

Experiment directly with the user's intuition:
- below roughly 1–3 px projected extent, aggressive aggregation may be acceptable if the resulting pixels remain correct;
- by roughly 4 px, more detailed representation should already exist.
Do not treat those numbers as architecture until measured.

## Distinguish three sizes

Never force the same unit to serve every layer:

1. **Logical tile** — spatial hierarchy/cache identity.
2. **Storage chunk** — CAS/dedup/compression unit.
3. **Microblock/workgroup** — GPU scheduling unit.

Initial benchmark:
- logical tiles: 64, 128, 256 (optionally 512);
- microblocks: 8×8 vs 16×16.

## Candidate semantic chunks

### OrbitSlab
Reference/formula/version, anchor, precision, iteration range, encoded states/checkpoints.

### BLAChunk
Dependencies, iteration ranges, coefficients, validity domain/radius, numeric format, error/remainder.

### ReturnMapChunk
Experimental: anchor/scale, branch/period/return description, local coefficients, certified domain/remainder, dependencies.

### CertificateChunk
Exact spatial region, claim type, error bound, validity range, dependencies.

### ExactSampleChunk
Exact local/dyadic coordinates plus classification/scalars and precision/error contract.

### PreviewChunk
Optional coarse raster/scalar result. Never canonical truth unless separately certified.

### TileManifest
Logical tile key plus hashes for chunks, child references, and available quality/certificate states.

### FrameManifest
Camera transform, output/sampling configuration, and references to tile/manifests. It should be tiny.

## GPU direction

Prefer structure-of-arrays for active state:
- delta real/imag;
- iteration/reference index;
- scale exponent;
- flags;
- derivative/DE state.

When lanes finish or fail an operator, compact surviving IDs into a smaller active queue rather than forcing finished lanes through pathological neighbors.

## Path-aware reuse

### Reference placement
Do not assume one reference per frame/tile.

For each candidate reference estimate:
- build seconds;
- atlas bytes;
- validity region;
- future pixels/frames accelerated;
- overlap with existing references.

Choose references that maximize future work avoided per byte and precompute second.

### Scheduling
Each chunk has:
- dependency graph;
- first-use frame;
- estimated remaining build cost;
- expected work saved.

Schedule by deadline/slack, not simply spatial distance.
The implemented scheduler (jobs, deadlines, cost model, `fd schedule`) is docs/spec/SCHEDULE.md;
the compiler built on it (`fd compile`, Atlas v0) is docs/spec/COMPILE.md.
Current state and next steps: docs/spec/STATE.md.
A local browser explorer (`fd explore`, a dev-tool sidequest) is docs/spec/EXPLORE.md.

### Temporal reuse
Keep distinct:
- exact sample reuse;
- operator/reference reuse for new sample points;
- raster reprojection for preview only.

## Acceleration ladder

### Gate 0 — trustworthy baseline
Build a genuine scalar renderer and oracle/benchmark harness first.

### Gate 1 — perturbation/local coordinates
Move deep absolute precision outside the normal per-pixel hot loop.

### Gate 2 — series/BLA
Represent many raw iterations with reusable operators.

### Gate 3 — certified BLA
Attach conservative remainder/error bounds so the same data can accelerate and inform LOD.

### Gate 4 — higher-order operators
Test second/higher-order local maps for larger validity domains.

### Gate 5 — periodic/first-return maps
High-risk/high-reward experiment: compile a high-period region so thousands/millions of raw steps become a small reusable operator.

Measure as period grows:
- compile time;
- operator bytes;
- validity region;
- evaluations/pixel;
- error;
- fallback rate;
- warm frame time.

Success means online cost grows dramatically slower than raw period.

### Gate 6 — specialized parabolic/renormalization maps
Only after earlier gates are measured.

## Milestones

### Gate A — trustworthy baseline
Baseline outputs, renderer, benchmark harness, exact address model.

### Gate B — atlas plumbing
CAS, tile/frame manifests, byte budget, known-path footprint planner.

### Gate C — “Google Maps effect”
References/operators reused across frames; LOD skips resolved work; atlas renderer beats independent frames.

### Gate D — depth flattening
Warm per-frame cost remains approximately stable over a substantial increase in absolute depth on at least one nontrivial genuine path.

### Gate E — novel compression
Long dynamical histories become much smaller reusable operators whose online cost grows far slower than raw iteration/period count.

### Gate F — interactivity
Only after video path reuse is proven.

## Open technical questions

- Best outer tile size on target hardware?
- 8×8 or 16×16 microblocks?
- How should coordinate/classification/scalar/shading errors combine into one refinement rule?
- Can useful boundary microblocks be certified as regions?
- How often can one reference serve multiple frames?
- How much does CAS dedup save?
- Best reference slab size/encoding?
- Is BLA GPU-friendly enough, or primarily CPU-side?
- Do higher-order maps justify their build/storage cost?
- Can return maps compress high-period paths by orders of magnitude?
- How small are their valid domains?
- Can interval/ball proofs be compact enough to store?
- Can a nested/dyadic sampling lattice increase exact temporal reuse?
- How much lookahead guarantees deadlines?
- Can atlas construction remain close to linear in movie length?
- Which visually rich paths are mathematically compressible?
- What classes of path remain hostile?
