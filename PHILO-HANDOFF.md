# PHILO-HANDOFF: carrying on the conversation

This is not a task list. The work state is in `docs/spec/STATE.md` plus Ishoo. This file
records the shape of the long conversation of 2026-10-07/08 between the owner and Claude:
- how it went;
- what keeps recurring;
- what the owner cares about;
- the mistakes not to repeat.

Read it to pick up the *conversation*, then read STATE.md to pick up the *work*. Then call
`ishoo_status`.

---

## The one-paragraph version

The atlas machinery works, but Gate C said it caches nothing worth caching (it ties a
per-frame BLA renderer). The search for real leverage found that **deep zooms near a
minibrot are governed by exact theorems** (Tan Lei, Koenigs). Turned into a per-pixel
pipeline, that makes deep frames **8.1-25x faster than our BLA renderer with every
pixel correct**: 0 wrong out of 777,600, measured on GitHub Actions three times
(PROB-08). The pipeline is:
1. a few return-map loops;
2. 23 approach steps;
3. one Koenigs jump;
4. 56-342 plain steps.

All of it is in ordinary doubles. The owner's view: "deep zooms being faster IS the win".
Next: skip the remaining ~300 steps (PROB-09), prove it on the whole v0 film (PROB-10),
and put it into fd so the videos benefit (KERN-01).

## Who the owner is (as observed)

- Built a large amount in about 24 hours with agents: a full Atlas v0 pipeline, then
  asked for an honest assessment.
- **Wants plain language.** They said "I can't read all that dude" and "explain simply,
  layman's terms". Lead with the answer in 2-4 sentences, then a short list or table.
  Avoid jargon, or translate it the first time it appears. Long walls of math lose them.
- **Wants momentum and testing, not hedging or giving up.** Their sharpest pushback:
  "you just want to give up? … whatever happened to our research method … testing lots
  of things very quickly and the real shit reveals itself." When a variant fails,
  generate and test the next one (in seconds) before drawing conclusions.
- **Cares about the deep-zoom win, not completeness.** When Claude listed "shallower
  frames still need other methods" as a limitation, the owner said "who cares though,
  deep zooms being faster IS the win". Don't pad results with caveats that don't matter
  to the goal. State real limits once, briefly.
- **Gets frustrated by apparent waste.** After the Gate C "no": "so all that work, for
  NOTHING". Reframe honestly (what was proven, what was not tested, what the
  infrastructure enables) without false cheer.
- **Has a creative, intuition-driven style.** "Google Maps + fractals", "1-3 px
  smooshed features", "constraints increase creativity". They believe AI innovates best
  under constraints and forced cross-domain analogies, and they test their human
  intuition through the AI.
- **Uses ChatGPT web for deep research** and likes being given 2-3 focused questions to
  run. The answers land in `docs/research/<date>/`. They volunteer for it ("if you want
  to give me a deep research question feel free ill run that for you").
- **Likes compute running while away.** "You can run more stuff in github actions while
  im gone … as much as you want." GitHub Actions is now the standard place for heavy or
  long runs (the `koenigs-bench` workflow).
- **Watches the session rate limit.** Mid-session: "dont talk too much or read too much
  just set it to go you are about to hit rate limit". When they say that, act in as few
  tool calls and words as possible.
- **Enjoys the payoff:** videos, the explorer, "how DOES he do it". It is a passion
  project. They are happy for others to copy it, or to stay mysterious and make videos
  people drool over. Honest scale: "front-page-HN and niche legend at best case".
- **Precise about instructions.**
  - "Put the files in Downloads/mandelbrot" meant *only the videos*.
  - "Do nothing else" means exactly that.
  - "Write a handoff and make what you ask it to do a filed issue" means file an Ishoo
    issue and point the handoff at it.
- Expects everything **filed** (Ishoo issues, ADRs, docs) so conversations can be
  dropped without losing anything.
- Contact emails (from CLAUDE.md): the owner's is junovhs@gmail.com; Ishoo's public
  address is hello@strangesystems.dev.

## Phases of the conversation (plot)

1. **Assessment.** Atlas v0 machinery complete. BENC-01 (Gate C) answered **no**: the
   atlas ties a per-frame-BLA renderer (0.996x) because it cached cheap things. The
   owner despaired; Claude reframed it as "narrower than it sounds".
2. **Fun interlude: a video.** A 10 s twist zoom into seahorse valley, copied to
   Downloads/mandelbrot.
3. **Sidequest: the explorer.** `fd explore` plus `viewer/explore.cmd`. The owner loved
   it.
4. **Method.** METHOD.md, DEC-14 (fast probes against a fair opponent), the north-star
   test "Earn it fast".
5. **Deep research rounds.** Q1 state of the art, Q2 return maps, Q3 cross-frame reuse.
   They led to FIX-03/FIX-04 and to probes.
6. **"Find me a REAL lead."** Claude killed its own first idea, then found
   **Misiurewicz frame transfer**: by Tan Lei's theorem, deep frames near a Misiurewicz
   point are exact rotated/scaled copies of shallower ones.
7. **Minibrot band decomposition.** Every deep pixel = k loops around the minibrot
   (period 764) + a getaway that depends only on the exit point.
8. **The break-test cycle:**
   - PROB-03 killed a fixed loop map.
   - PROB-07 kept a tight-exit degree-4 map.
   - PROB-04 killed the interpolated exit table (the getaway is itself a fractal).
   - Claude got defeatist and the owner pushed back. Claude found the **Koenigs jump**:
     an exact skip of the getaway's spiral-out (FIX-13).
9. **This session's run (2026-10-07 afternoon/evening):**
   - **Leftovers diagnosed** (FIX-15): the ~300 steps left after the jump are the
     near-parabolic "seahorse gate" around c = −3/4, not a second clean spiral.
   - **Everything in plain doubles** (FIX-15, FIX-16): the whole pixel needs no
     multiprecision. Degree 2/3/4 loops and guard sweep recorded.
   - **Real timing in Rust on GitHub Actions** (PROB-08): 8.5-23x vs fd per-frame BLA,
     every pixel correct. A lean perturbation baseline was added to rule out a
     "lean code vs bloated code" artefact; fd beat it, so fd is a fair opponent.
   - **Repeat runs** (FIX-18): 4 threads 8.7-25x, 1 thread 8.1-25x. It is per core,
     not a threading effect.
   - **Deep-research report** on near-parabolic skipping, filed. Exact methods exist,
     and no renderer uses them per pixel yet.
   - **Filed next steps:** PROB-09, PROB-10, PROB-11, KERN-01 and DEC-17 (proposed).

## Recurring themes and tropes

- **"Is it dead?" followed by "no, the test was narrower than the claim."** Gate C,
  PROB-03, PROB-04. Check whether the test design was the real failure before calling
  something dead. But don't oversell either.
- **Overclaiming, then correcting.** State size estimates as ranges, with what they
  depend on, *before* the excitement. This session got it right by insisting on real
  timing before claiming the 10x.
- **Sparse tests lie.** Two configurations passed 48 frozen points and then failed whole
  frames (rare bad pixels). Keep only after every pixel of whole frames matches
  (DEC-17, proposed; METHOD.md §4).
- **Check the worst pixels against high-precision truth.** When the candidate and fd
  disagreed, 90-digit direct iteration showed fd exact and the candidate off. That told
  us which side to fix.
- **The fair-opponent rule** (DEC-14, DEC-15). Per-frame tricks make frames cheaper for
  *everyone*; label every result as "cheaper frames" or "atlas win". The Koenigs pipeline
  is a "cheaper frames" win.
- **Kill fast, write it down.** Every result goes into the METHOD.md results log with
  command and numbers.
  - Killed so far: analytic exterior patches; minibrot-as-scaled-whole-set; the naive z²
    map; one fixed return map over the wide schedule; uniform bilinear exit tables;
    degree-2 loops (fail whole frames); guard 1e-26 (fails whole frames); guards ≤ 1e-29.
- **"Holy shit" cross-domain connections** are what the owner wants. The winning
  pattern: *a classical theorem gives an exact transform, and the renderer exploits it
  under an error contract.* Tan Lei gave transfer; Koenigs gave the jump; next could be
  Kapiamba/Fatou coordinates for the gate.
- **Genuine depth is sacred** (DEC-01). Theorem-backed transfer is argued to be genuine
  (DEC-16, **still PROPOSED: the owner must decide**). The Koenigs pipeline computes the
  real pixel, so it raises no such question.
- **Precision traps** keep biting test scripts:
  - mpmath constants parsed before `mp.dps` is set;
  - off-by-one in iteration indices (twice this session: ν off by exactly 1.0);
  - rounding a near-C value straight to double (ζ − C is 1e-26, so ζ rounds to C; pass
    offsets, not values);
  - Windows case-insensitive filenames;
  - Windows Python writing files with cp1252 (use `PYTHONUTF8=1` / `encoding='utf-8'`,
    or a failed write truncates the file).
  Results that look *too perfect* or *uniformly broken* are usually script bugs.
- **Error metric:** pixel displacement = |Δν|·ln2·de, with de in output pixels (fd's de
  column). The bar is ≤1e-3 px.

## Key numbers to remember

- **v0 path:**
  - 750 frames, width 4 to 2e-49.
  - Centre: a period-764 minibrot nucleus C, 1.7e-25 from the Misiurewicz point M(24,2).
  - 2-cycle multiplier ρ = 1.02683+0.52496i (|ρ| = 1.1532).
  - Minibrot size about 4.12e-50.
- **Seahorse gate:**
  - α fixed point of f_C at about −0.4988+0.0657i, multiplier λ = −0.9976+0.1314i,
    |λ| = 1.00622 (barely repelling).
  - Canonical gate width about 22 raw steps (1/α_NP ≈ 11.47 − 0.57i, from the report).
- **BENC-01:** plain 647 s, per-frame BLA 478 s, atlas 480 s. 99.3% of time is
  per-pixel iteration.
- **PROB-08** (GitHub Actions, AMD EPYC, 480x270, best of runs), seconds per frame:

  | width | fd per-frame BLA | Koenigs pipeline | speed-up |
  |---|---|---|---|
  | 1e-35 | 0.51 | 0.060 | 8.5x |
  | 1e-38 | 0.84 | 0.082 | 10.3x |
  | 1e-40 | 1.37 | 0.059 | 23.4x |
  | 1e-43 | 0.68 | 0.071 | 9.6x |
  | 1e-46 | 0.73 | 0.071 | 10.2x |
  | 2e-48 | 0.78 | 0.070 | 11.2x |

  Single thread: 8.1-25x. Against the lean perturbation baseline: 11-37x.
- **fd at these depths:** about 690-775 iterations per pixel after about 37-81 BLA blocks.
  Most of fd's time is plain steps in the gate region that BLA cannot skip.
- **Pipeline settings:** degree-4 biseries (14 terms), guard 1e-28 (1e-27 fails at 1080p), φ 12 terms,
  R0 0.03, ψ 18-term series, tail patch atlas depth 6.
  Per-zone constants take about 0.1 s in Python.
- Reference cost is about 4% of a frame even at 1e-1000, so caching references never
  wins.

## Where things live

- **Probes:** `tools/research/misiurewicz/`
  - `koenigs_tail.py`: Koenigs jump, mpmath;
  - `koenigs_double.py`: tail in double;
  - `koenigs_double_full.py`: whole pixel in double;
  - `koenigs_leftover.py`: gate diagnostic;
  - `returns_exit_tail.py`: biseries, PROB-03/04/07;
  - `return_map_truth.json`: frozen 48 points.
- **Rust bench:** `tools/research/misiurewicz/koenigs_bench/`
  - `src/main.rs`: modes `koenigs` and `perturb`;
  - `run.sh`, `compare.py`, `report.py`;
  - `../koenigs_bench_consts.py`.
- **CI:** `.github/workflows/koenigs-bench.yml` (manual: `gh workflow run
  koenigs-bench.yml -f threads=4 -f runs=5`). The report is on the run summary page and
  in the `koenigs-bench` artifact. The main `ci.yml` runs on every push.
- **Write-ups:** `docs/research/10-8-26/misiurewicz-frame-transfer.md` (the whole
  story), `docs/research/10-8-26/skipping-near-parabolic-transits-what-is-computable.md`
  (deep-research report), METHOD.md results log.

## How to work with the owner

- Answer first, plainly. Use a table for numbers. Offer the next quick test. Don't stop
  at "maybe it's dead".
- Use Ishoo MCP tools for all issue, ADR and plan work. Call `ishoo_brief` at session
  start. Small research results land with `ishoo_hotfix`; planned work uses
  `ishoo_new` → `ishoo_start` (worktree) → `ishoo_resolve` → `ishoo_done`.
  - A `workflow_dispatch` workflow can only be triggered once its file is on `main`.
    Land it first, then dispatch, then record the CI numbers in a follow-up hotfix.
- Research probes go in `tools/research/`, with results in METHOD.md and the research
  write-up.
- Before a new line of investigation, draft 2-3 deep-research questions for the owner.
- Ask before outward or irreversible actions (git config changes, deleting things).
  Running benchmark workflows on GitHub Actions is pre-approved by the owner.
- When the owner signals the rate limit, minimise tool calls and words.

## Where to pick up

1. **Check the GitHub Actions runs queued while the owner was at lunch** (`gh run list
   --workflow koenigs-bench.yml`). Fetch each report (`gh run download <id> -n
   koenigs-bench`) and log the numbers in METHOD.md.
2. **Done: PROB-09.** The tails are not an α dwell. The jump's Newton ψ was 60% of the
   pixel; ψ as a series plus a per-zone tail patch atlas (16k Taylor patches, 4 MB) gives
   **20-54x vs fd at 1080p, 0 wrong of 12.4M pixels**. FIX-20's cause was the 12-term
   Newton inverse; series ψ fixes it. CI: `-f psi=18 -f patch=6`. The owner was given a
   single deep-research question on automatic zone detection and chaining along a whole
   path (for KERN-01 and long 8K films); its report goes in `docs/research/`.
3. **PROB-10:** the whole v0 film's band frames at full resolution, every pixel, total
   time, on GitHub Actions.
4. **KERN-01:** put the fast path into fd (render/control/play) so films get faster.
5. **Owner decisions pending:** DEC-16 (theorem-backed transfer = genuine depth) and
   DEC-17 (whole-frame validation before "keep").
6. **Deep-research questions offered and not yet answered:**
   - Q2: is the "loops + Koenigs tail" pipeline already known in Kalles
     Fraktaler/Imagina/NanoMB/mathr?
   - Q3: automatically finding the nucleus/Misiurewicz point/cycle for any location,
     plus a cheap c ≠ C tail correction.
   Q1 (near-parabolic skipping) was answered and filed.
7. **Also open:**
   - PROB-05 (Misiurewicz-zone frame transfer);
   - PROB-06 (nested minibrots, automatic zone detection);
   - PROB-11 (off-centre/shallower frames, mid: the owner rates it low);
   - FIX-03/04/09;
   - TRUT-01/PROB-01.
