# Mandatory prior-art search and evidence ledger

**Hard gate for all NEW research questions and experiments after 2026-10-12.** Applicable to this documentation-only research branch, including incremental optimizations and follow-ups. See [operating rules](AGENTS.md), [method](METHOD.md), [P template](TEMPLATES/QUESTION.md), [R template](TEMPLATES/RESULT.md), and [proof-of-done](PROOF_OF_DONE.md).

## Search -> decide -> pivot or proceed

**Prior-art review is an elimination filter, not an invitation to try to outperform the known method.**

1. **Search before proposing to compute.** Formulate a candidate mathematical mechanism, search for its published mathematics and application, and inspect a credible primary abstract/paper when the match is close. If prior art already contains the substantive mechanism for materially the same problem, **STOP the candidate immediately**. Record the exact published title/link/DOI, query (if searched), what overlaps and why that settles the proposed question in [FRONTIER.md](FRONTIER.md) and [SOURCES.md](SOURCES.md), or in an existing P/PA record. **PIVOT to a different hypothesis** (another structure, mathematical property or cross-domain gap); do not benchmark, retune or vary polynomial degree simply because the method might be faster. A decisive overlap may stop a *search-only* candidate before three queries; no P-card, R-card or POD is needed for an unrun idea.

2. **Only if the candidate survives that filter**, execute at least **three distinct external searches**: mechanism/synonyms, application/domain and strongest specialized alternatives. Read a directly relevant primary abstract/paper. Preserve all exact executed queries with UTC date/provider, actual returned titles/authors/dates/URLs/DOIs, close and negative matches, limitations and at least three relevant result records where available. If fewer, document the actual shortfall. Do not invent search results, inaccessible texts, or claims of originality. An absence of matches is not proof of novelty.

3. **Document the explicit decision before preregistration:**
   - `TRIAGE: PIVOT` (**default on known overlap**): published work addresses the same essential technique and target; no substantive distinct open gap is supported. **No numerical test**.
   - `TRIAGE: PROCEED`: clearly name a genuinely unanswered mechanism, theorem, correctness boundary or materially different application and why the closest work does not settle it. Calling it a "faster version" alone is not enough. `KNOWN_METHOD_NEW_APPLICATION` needs a substantive unaddressed question.
   - `TRIAGE: OWNER-REQUESTED EXCEPTION`: the owner explicitly asks to reproduce, calibrate, benchmark or engineer a known algorithm; non-novel scope and purpose are stated. Do **not** self-authorize known-method speed races.
   - `TRIAGE: BLOCKED`: necessary external verification is unavailable, ambiguous or insufficient.
   The existing labels `KNOWN_METHOD_BENCHMARK`, `KNOWN_METHOD_NEW_APPLICATION`, `OPEN_GAP_HYPOTHESIS`, and `NOVELTY_UNVERIFIED` describe evidence and overlap, **not permission to execute**. A known-method benchmark is an exception, not the lab's default output.

4. **Only when `PROCEED` or owner exception applies**, create the complete [dossier](PRIOR_ART/TEMPLATE.md) at `PRIOR_ART/PA-P-NNN.md` plus the ≤150-word `PROMPTS/P-NNN.md` in the **same pretest commit**. Both link each other and state exact searches, primary overlap, claim classification, triage decision, precise delta (or authorized replication purpose), strongest fair rival, and frozen gate. Read back both files and branch head **before running code**. Inability to search/commit/read back → `PRIOR_ART: BLOCKED`, no experiment.

**Citing a paper and pivoting is useful research in itself.** Do not turn that discovery into an unnecessary numerical contest simply to fill the experiment ledger. Prior-art methods become benchmark opponents only *after* a distinct research question has justified a test.

## When writing an R-card / reporting to the owner

- Link the P-card's **pre-experiment prior-art Git commit** and dossier; report search query provenance, primary sources reviewed, claim class and unresolved difference. Recheck literature if the work unexpectedly resembles a known method; file corrected findings as an amendment, and **downgrade novelty claims promptly**.
- **Do not promote an overlap into an experiment merely by calling it a benchmark.** A known-method speed contest without an independent unresolved question requires explicit owner direction. **Never** describe an observed speedup as a newly discovered algorithm when its underlying method is established. “Known technique; new scoped measurement,” “independent replication,” and “candidate extension not yet shown in literature” are valid, distinct results.
- New R-cards require the **10/10 [proof-of-done](PROOF_OF_DONE.md)**, including the auditable literature search. A completed numerical result with missing search is **POD: NO**; numerical `GATE: PASS` does not confer originality. The archival evidence classification (`REPORTED/REPRODUCED/CERTIFIED`) is separate from the *novelty/prior-art* class.
- The dossier is **part of the committed research output**, not merely a citation list or a private chat/search history. Search timestamps and result URLs must survive conversation resets.

## Retrospective example: how the prior failure happened

The [PWM records R-006–R-008](RESULTS/R-008.md) tested a Taylor-polynomial kick on a switched-converter surrogate and only **afterward** identified prior work on polynomial state-transition approximations for buck converters (Luciano and Strollo, **1990**). The [retrospective search record](PRIOR_ART/PA-PWM-RETRO.md) records the actual search queries, results and direct overlap. **It was not a pretest search**, so it cannot repair the old workflow retrospectively. R-008 remains a scoped reported 1.209× benchmark result **using established numerical ideas**, not an invention claim.

**P-083/R-009 correction:** [P-083](PROMPTS/P-083.md) *did* search beforehand and found published hybrid-event localization. It then unnecessarily ran a quadratic predictor versus Newton speed contest anyway. [R-009](RESULTS/R-009.md)'s numerical observations remain intact as historical data. **The intended response to that search was: "already known; cite it; what else?"** not another benchmark. Its known-method discovery follow-up is closed.

**Historical P-000–P-082 / R-001–R-008** retain their original gate/history and nine-box audit, without false pretenses of pretest literature diligence. However, **any future experiment**, including continuation of an old open lead such as P-030, P-040, P-051, P-060 or P-070, must first register a **new linked P-ID and its pretest dossier**. A purely clerical update to old results is not a new experiment.
