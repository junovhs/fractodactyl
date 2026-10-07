# PHILO-HANDOFF: carrying on the conversation

This is not a task list (that is `docs/spec/STATE.md` plus Ishoo). It records the shape of
the long conversation of 2026-10-07/08 between the owner and Claude: how it went, what
keeps recurring, what the owner cares about, and the mistakes not to repeat. Read it to
pick up the *conversation*, then read STATE.md to pick up the *work*.

---

## Who the owner is (as observed)

- Built a large amount in about 24 hours with agents: a full Atlas v0 pipeline, then
  asked for an honest assessment.
- **Wants plain language.** They said "I can't read all that dude" and "explain simply,
  layman's terms". Lead with the answer in 2-4 sentences, then a short list. Avoid
  jargon, or translate it the first time it appears. Long walls of math lose them.
- **Wants momentum and testing, not hedging or giving up.** Their sharpest pushback:
  "you just want to give up? … whatever happened to our research method … testing lots
  of things very quickly and the real shit reveals itself." When a variant fails,
  generate and test the next one (in seconds) before drawing conclusions.
- **Gets frustrated by apparent waste.** After the Gate C "no": "so all that work, for
  NOTHING". Reframe honestly (what was proven, what was not tested, what the
  infrastructure enables) without false cheer.
- **Has a creative, intuition-driven style.** "Google Maps + fractals", "1-3 px
  smooshed features", "constraints increase creativity". They believe AI innovates best
  under constraints and forced cross-domain analogies, and they test their human
  intuition through the AI.
- **Uses ChatGPT web for deep research** and likes being given 2-3 focused questions to
  run. The answers land in `docs/research/<date>/`.
- **Enjoys the payoff:** videos, the explorer, "how DOES he do it". It is a passion
  project. They are happy for others to copy it ("if they want to copy me and do it
  better, sweet"), or to stay mysterious and make videos people drool over. They asked
  "how big is this, Nobel or 100 HN upvotes?"; the honest answer was "front-page-HN and
  niche legend at best case".
- **Precise about instructions.**
  - "Put the files in Downloads/mandelbrot" meant *only the videos*. Putting the
    renderer and launcher there was wrong, and they corrected it.
  - "Do nothing else" means exactly that.
  - "Write a handoff and make what you ask it to do a filed issue" means file an Ishoo
    issue and point the handoff at it.
- Expects everything **filed** (Ishoo issues, ADRs, docs) so conversations can be
  dropped without losing anything.

## Phases of the conversation (plot)

1. **Assessment.** Reviewed the repo: Atlas v0 machinery is complete. BENC-01 (Gate C)
   answered **no**: the atlas ties a per-frame-BLA renderer (0.996x) because it cached
   cheap things (the reference and BLA cost 18 ms of a 640 ms frame). The owner
   despaired; Claude reframed it as "narrower than it sounds", not a verdict on the idea.
2. **Fun interlude: a video.** A 10 s, 24 fps, 720p twist zoom from the whole set into
   seahorse valley, rendered locally in about 4 min (3 looks). Played in VLC, copied to
   Downloads/mandelbrot.
3. **Sidequest: the explorer.** `fd explore` plus `viewer/explore.cmd`: wheel zoom,
   drag pan, Blender-style coarse-to-fine refinement, 10 presets down to 1e-1000,
   relief look. The owner loved it ("braaaavo bro").
4. **Method.** The owner pasted another agent's analysis. Claude critiqued it (the
   fair-opponent trap, cross-frame state reuse doesn't work naively, counting steps
   isn't timing). This led to METHOD.md, DEC-14 (fast probes against a fair opponent)
   and a north-star test, "Earn it fast". Conversation about how AI innovates:
   constraints, forced analogies, generate-and-test against fast scorers.
5. **Deep research rounds.** Three owner-run reports: Q1 state of the art (Imagina
   LA/AT, FloatExp BLA, atom-domain interior), Q2 return maps (NanoMB), Q3 cross-frame
   reuse (exponential maps, zoomasm). They led to fixes (FIX-03 BLA beyond 2^-900,
   FIX-04 deep interior) and to probes.
6. **"Find me a REAL lead."** Claude searched, killed its own first idea (analytic
   exterior patches: 0% of pixels far from the set on hard views), then found
   **Misiurewicz frame transfer**. By Tan Lei's theorem, deep frames near a Misiurewicz
   point are exact rotated/scaled copies of shallower ones: 0 wrong pixels, ≤1e-6 px.
   It is novel as a *rendering* technique.
7. **Minibrot band.** The naive z² copy failed. The correct decomposition: every deep
   pixel = k loops around the minibrot (764 iterations each) plus a getaway that
   depends only on the exit point. Exact.
8. **The break-test cycle,** run by fresh sessions with Claude reviewing:
   - **PROB-03** killed a fixed loop map; the test schedule was too wide.
   - **PROB-07** kept a tight-exit degree-4 map (0 wrong pixels, ≤2.2e-4 px).
   - **PROB-04** killed an interpolated exit table (1-27 px errors; the getaway is
     itself a fractal).
   - Claude started sounding defeatist and the owner pushed back. Claude then found the
     **Koenigs jump**: an exact closed-form skip of the getaway's spiral-out, with no
     table. 0 wrong pixels; the getaway falls from 113-996 to 56-342 steps.
9. **The current outlook (honest):** roughly 3-10x cheaper on the deep band, exact,
   with no table. That is still a projection; no real timing yet. The "50-100x at
   extreme depth" dream needs more wins. The owner wants rapid variant testing to
   continue.

## Recurring themes and tropes

- **"Is it dead?" followed by "no, the test was narrower than the claim."** This has
  happened repeatedly: Gate C, PROB-03, PROB-04. Before calling something dead, check
  whether the test design (exit schedule, metric, precision, opponent) was the real
  failure. But also don't oversell.
- **Overclaiming, then correcting.** Claude called the Misiurewicz lead "big", then had
  to downgrade it to "1.3x on its own". Size estimates should be stated as ranges, with
  what they depend on, *before* the excitement.
- **The fair-opponent rule** (DEC-14, DEC-15). Per-frame tricks (BLA, biseries, Koenigs
  jump, LOD) make frames cheaper for *everyone*; only expensive-to-rebuild or
  inherently cross-frame things are atlas wins. Always label which question a result
  answers.
- **Kill fast, write it down.** Every result, positive or negative, goes into the
  METHOD.md results log with command and numbers. Ideas killed so far: analytic
  exterior patches; the minibrot-as-scaled-whole-set similarity; the naive z² map; one
  fixed return map over the wide schedule; uniform bilinear exit tables.
- **"Holy shit" cross-domain connections** are what the owner wants: complex dynamics
  theorems (Tan Lei, Douady-Hubbard, Koenigs) combined with video-codec/atlas ideas. The
  winning pattern so far: *a classical theorem gives an exact transform, and the
  renderer exploits it under an error contract.*
- **Genuine depth is sacred.** The north star forbids look-alike substitution (the
  retired twin approach, DEC-01). Theorem-backed transfer is argued to be genuine
  (DEC-16, **still PROPOSED: the owner must decide**).
- **Precision traps** keep biting test scripts:
  - mpmath constants parsed before `mp.dps` is set;
  - off-by-one in iteration indices;
  - Windows case-insensitive filenames (`m.fds` == `M.fds`).
  Results that look *too perfect* or *uniformly broken* are usually script bugs, so
  check them before reporting.
- **Error metric:** pixel displacement = |Δν|·ln2·de/(width/480). The bar is ≤1e-3 px
  (a strict tool tolerance; LOD.md's display tolerance is 0.25 px, which is a possible
  debate for resampling-based methods like PROB-05).

## Key numbers to remember

- v0 path: 750 frames, width 4 to 2e-49, centre = a period-764 minibrot nucleus C,
  1.7e-25 from Misiurewicz point M(24,2). ρ = 1.02683+0.52496i, |ρ| = 1.1532, so the
  picture repeats every 0.0619 decades (the path moves 0.0657 decades per frame).
  Minibrot size λ ≈ 4.12e-50.
- BENC-01: plain 647 s, per-frame BLA 478 s, atlas 480 s. 99.3% of time is per-pixel
  iteration.
- Deep-band pixel costs:
  - direct: about 9,000-25,000 ops;
  - our BLA: about 10,000 modelled ops (BLA is 0.78-0.97x as fast as plain at 1e-14 to
    1e-29);
  - new pipeline: degree-4 loops at 240 ops each, k = 2-5, then the Koenigs jump
    (12 terms, R0 = 0.03), then 56-342 direct steps.
- Reference cost is only about 4% of a frame even at 1e-1000, so caching references
  never wins.

## How to work with the owner

- Answer first, plainly. Offer the next quick test. Don't stop at "maybe it's dead".
- Use Ishoo MCP tools for all issue, ADR and plan work. The Ishoo-managed repo's
  CLAUDE.md requires `ishoo_brief` at session start.
- Research probes go in `tools/research/`, with results in METHOD.md and
  `docs/research/10-8-26/misiurewicz-frame-transfer.md`.
- Before a new line of investigation, consider drafting 2-3 deep-research questions
  for the owner to run.
- Ask before outward or irreversible actions (git config changes, deleting things).
  Do exactly what is asked when the owner is precise.

## Where to pick up

The owner was offered (and had not yet answered) a run of quick variant probes:
1. why some getaways still need about 300 steps after the Koenigs jump (a second
   spiral to skip?);
2. ψ by series reversion instead of Newton;
3. degree-2 loops with a smaller guard (cheaper);
4. an exact-sample exit cache (no interpolation) to revisit the table idea properly;
5. **a real speed measurement** against per-frame BLA.

Also open: PROB-05 (Misiurewicz-zone prototype; decide exact-grid vs a 0.25 px
tolerance first), PROB-06 (nested minibrots, automatic detection), DEC-16 (the owner's
call), and the FIX/PROB backlog in STATE.md.
