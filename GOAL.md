# Standing goal: discover useful computational shortcuts

**This is the project's durable objective, not an assignment to one technique or industry.**

Discover, test, and eventually generalize mathematical methods that make **scientific or industrial computation more efficient**, especially where nonlinear dynamics contain exploitable structure. Fractodactyl supplied the first techniques and research discipline; rendering is **not** the goal.

The expected output is **findings**: promising unanswered mechanisms, credible new mathematical shortcuts, transferable gains, meaningful failures, previously measured incremental benefits and unexpected connections. **Finding that a proposed idea was already published is a successful early research result:** cite the source, record the precise overlap and investigate a *different* hypothesis without running it. Do not default to a "try to beat this known algorithm" project. Preserve legitimate historical speed measurements without letting them dictate discovery priorities.

**Mandatory discovery triage:** Search primary prior art **before deciding to run an experiment**. When the same core mechanism and materially similar application already exist, **STOP/PIVOT**, record the citation and explore a different mechanism, rather than relabeling an implementation race a new discovery. Only a substantive unresolved mathematical/application question passes to preregistration and fair performance comparisons; the owner may explicitly request non-novel replication/deployment work as an exception. Any new *experiment* still requires a pretest-committed, actually executed three-query dossier and linked P-card as in [PRIOR_ART.md](PRIOR_ART.md). Unavailable searches are **BLOCKED**; no matching hit alone never establishes novelty.

## Agent autonomy

Agents are **not required to follow a preselected technical roadmap** or keep pursuing power electronics. At each research session, independently read the evidence, propose competing leads across domains, eliminate already-known approaches, select a high-information *unresolved* idea, run an experiment only if the literature triage warrants it, and update the [frontier](FRONTIER.md). The owner need not know which algorithm to pursue. Follow evidence rather than fashion, hunches, or sunk cost.

Search broadly enough to find surprising connections, but demand a **substantive difference from published work**, a concrete mechanism, comparable baseline and cheap falsifiable probe before investing. A session can productively end with a cited literature elimination and a new direction, without any experiment. Preserve cross-domain transfer opportunities; avoid both aimless brainstorming and premature commitment to one application.

## How we recognize a valuable discovery

- **Usefulness:** a plausible real workload (science, industry, computation), not merely a pretty demonstration.
- **Transfer:** a mechanism that plausibly works beyond a handcrafted fractal coordinate or single toy model.
- **Truthfulness:** independent accuracy checks first, rigorous guarantees where needed; no overstated certification.
- **Economics:** **no mandatory speedup floor—not 3×, not 1.10×.** A reliable 1% gain can be meaningful in a heavily used computation; a 10%, 30%, or 2× gain deserves to be preserved. Measure a realistic workload against the strongest accurate same-math/specialist rival, with startup, warm reuse, amortization and absolute time saved clearly distinguished. Allocate further research effort by expected cumulative saved compute, applicability, uncertainty and implementation cost; treat 3× as an ambitious breakthrough tier, never a universal survival gate.
- **Autonomy:** hypotheses and next experiments are selected from evidence; all tests and dead ends are recorded for other agents.

**Two independent questions:** (1) Is a gain accurate, repeatable and fairly measured? (2) Does its expected real-world payoff justify more investigation? Do not erase a genuine positive effect merely because it misses a predeclared experimental speed target; preserve the original `GATE: FAIL` and record the narrower opportunity for a new preregistered test. Likewise, a small gain may be real but not a priority to optimize further. Existing P-card numerical thresholds remain historical facts, not defaults for new work.

A general-purpose mathematical compiler is a **research aspiration, not an assumed feasible outcome**. We may discover narrower valuable algorithms instead.

[Dynamic frontier](FRONTIER.md) · [Operating rules](AGENTS.md) · [Binary proof of done](PROOF_OF_DONE.md) · [Research map](MAP.md).
