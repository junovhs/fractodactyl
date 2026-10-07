# Fractadactyl Atlas — Authoritative Reset Spec

**Status:** authoritative project reset  
**Date:** 2026-10-06  
**Historical prototype:** `ishoo/POC-02` @ `790be5b6f42e4525cc71f794aa044ff1c57b7666`

## North star

Render the **genuine Mandelbrot set at genuine extreme depth** while making warm per-frame cost grow as little as possible with absolute depth.

Fractadactyl should behave more like a mathematical Google Maps engine than a conventional independent-frame renderer: compile the known camera corridor ahead of time, cache reusable mathematical work, refine detail before it becomes visible, and render later frames from a bounded atlas.

## Why the old architecture is retired

The twin-minibrot approach replaced a deep feature with a visually similar shallow one. It produced useful evidence but has two structural problems:

1. seams and decoration-density mismatches become visible as the substituted region grows;
2. repeated shallow substitutions risk making long movies feel synthetic or repetitive.

The old implementation remains historical evidence. Do not rebuild it as the default architecture.

## Locked principles

### 1. Genuine rendering only
Final pixels must correspond to the requested Mandelbrot coordinates, subject to explicit numerical approximation/error contracts. Local coordinates, perturbation, BLA, series, return maps, cached references, and certified approximations are allowed. Location substitution is not.

### 2. Optimize warm path cost
Depth-dependent work may happen during route analysis, reference generation, atlas construction, certification, and cache population. The primary target is warm sequential/video rendering, not arbitrary cold frames.

### 3. Bounded path-specific atlas
Planning target: **2.5 GiB**. Default hard ceiling: **3.0 GiB**. The atlas covers one known path plus a modest envelope and fallback ancestors; it never attempts to precompute the whole Mandelbrot set.

### 4. Cache computation, not pictures
Canonical reusable payloads are mathematical: local transforms, reference orbit chunks, iteration-skipping operators, validity domains, certificates, and exact samples. Raster previews are optional latency masks, not canonical truth.

### 5. Screen-space deadlines
Only compute detail the current raster can reveal. A coarse representation may remain valid while its projected uncertainty is below the visual/numerical threshold. Refine before that uncertainty becomes resolvable. The user's 1–3 px “smooshed” intuition and ~4 px readiness point should be tested, not hard-coded.

### 6. Separate math from appearance
Deep computation should produce palette-independent outputs such as classification, smooth escape/potential values, derivative/distance-related values, normals, and uncertainty. Palette/lighting/relief are a cheap late pass.

### 7. Content-addressed immutable chunks
Frames and tiles reference shared chunks by hash. Identical semantic chunks are stored once.

### 8. Exact hierarchical addressing
Use exact parent/child relationships, preferably dyadic, and local coordinates so deep pixels do not carry enormous global coordinates through the hot loop.

### 9. Rust production engine
Rust is the default implementation language. Python may be used as an oracle/research environment, not as the production hot path.

### 10. Every acceleration has an error contract
For every shortcut define: what it approximates, where it is valid, how much error it may introduce, how failure is detected, and what fallback runs.

## Provisional starting assumptions

These are experiments, not permanent decisions:

- 128×128 outer logical/cache tiles.
- 8×8 or 16×16 GPU microblocks.
- Quadtree spatial hierarchy with dyadic coordinates.
- Fixed semantic reference slabs (for example 2K/4K/8K iterations).
- BLA as the first reusable acceleration payload.
- Offline/CPU certificate generation with cheap GPU consumption.
- ~0.25 px projected uncertainty as an initial final-video target.

## System split

### Atlas compiler
Input: camera path, resolution, sampling policy, quality/error target, atlas budget, renderer/formula version.

Responsibilities:
- predict future view footprints;
- choose logical tiles;
- generate/reuse references;
- build operators;
- certify validity;
- schedule preparation by first-use deadline and build cost;
- deduplicate chunks;
- emit manifests;
- enforce atlas budget.

### Atlas renderer/player
Input: frame camera state, atlas manifests/chunks, appearance settings.

Responsibilities:
- resolve visible tile/microblock working set;
- execute cheap certified operator paths;
- send failures to fallback queues;
- produce palette-independent scalar/sample results;
- shade/color/encode.

## First decisive question

Before UI polish or exotic research, prove or disprove:

> Can a bounded path-specific mathematical atlas make later genuine deep-zoom frames reuse enough prior computation to become materially cheaper than rendering each frame independently?

## Atlas v0 success criteria

1. Render a genuine 20–30 second Mandelbrot zoom.
2. No fake-location substitution.
3. Stay under the configured 3 GiB ceiling.
4. Reuse expensive mathematical chunks across multiple frames.
5. Beat independent-frame rendering after precomputation on the same path.
6. Recolor/re-light without rebuilding the mathematical atlas.
7. Record fallback and error metrics.
8. Identify the next bottleneck from measurements.

## Required metrics

Compiler:
- atlas bytes and bytes by chunk type;
- unique chunks and dedup ratio;
- reference/operator/certification time;
- compiler peak RAM;
- reuse counts.

Renderer:
- warm and cold seconds/frame;
- bytes read/frame;
- tiles/microblocks touched;
- macro-operators/pixel;
- fallback iterations/pixel;
- fallback pixel fraction;
- peak RAM/VRAM.

Correctness:
- classification mismatches vs oracle;
- scalar error vs oracle;
- accepted projected uncertainty;
- certificate failures/fallbacks.

Scaling:
- absolute depth;
- relevant period/reference length;
- atlas bytes per minute/octave;
- compile seconds per minute/octave;
- warm-cost slope with depth.

## Falsification criteria

Treat these as valid research outcomes:
- atlas growth exceeds the path budget;
- most pixels repeatedly leave cached validity domains;
- operator/reference traffic costs more than direct arithmetic;
- GPU divergence erases saved math;
- certification costs more than it saves;
- screen-space LOD cannot safely defer work;
- warm cost still climbs strongly with depth;
- return-map payloads scale with raw orbit history;
- 2–3 GiB only supports trivial paths.

## Read next

- `docs/spec/ARCHITECTURE.md` — tile/storage/GPU/LOD design and open technical questions.
- `docs/spec/ISHOO-LEDGER.md` — exact ADR/issue/plan structure to file locally.
- `docs/research/10-6-26/01-depth-independent-per-frame-cost.md`
- `docs/research/10-6-26/02-google-maps-mathematical-atlas.md`
- `docs/research/10-8-26/zoom-computation-reuse.md` (cross-frame reuse, exponential maps)
- `docs/research/10-8-26/minibrot-renormalization-local-maps.md` (return maps near minibrots)
