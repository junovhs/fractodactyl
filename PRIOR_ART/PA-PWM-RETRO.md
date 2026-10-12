# PA-PWM-RETRO — After-the-fact prior-art search for P-080 / P-081 / P-082

**Status: RETROSPECTIVE — DID NOT PRECEDE THESE EXPERIMENTS.**
**Actual search date:** 2026-10-12 (UTC date, current conversation)
**Search service:** External web search; source discovery from indexed publisher/institutional search results. Direct full text not obtained for every hit.
**Related original hypotheses:** [P-080](../PROMPTS/P-080.md), [P-081](../PROMPTS/P-081.md), [P-082](../PROMPTS/P-082.md)
**Related measurements:** [R-006](../RESULTS/R-006.md), [R-007](../RESULTS/R-007.md), [R-008](../RESULTS/R-008.md)
**Prior-art process:** **FAILED BEFORE THE ORIGINAL PROBES**. This is a correction, **not** pre-registration or a demonstration of novelty.
**Correct classification:** `KNOWN_METHOD_BENCHMARK` (with unverified implementation-specific performance on a synthetic workload).

## Literal queries actually executed (in order)

Search 1 (mechanism / authors), exact literal text:
> Luciano Strollo 1990 fast time domain algorithm simulation switching power converters Chebyshev state transition matrices DOI

Search 2 (application / method synonyms), exact literal text:
> polynomial approximation state transition matrix switched dc dc buck converter fast simulation Chebyshev 1990 paper

Search 3 (related numerical methods / optimized rivals), exact literal text:
> computing matrix exponential optimized Taylor polynomial approximation 2019 paper scientific computing

**All three were actually submitted to the external search tool on 2026-10-12.** Results below are representative relevant hits **returned**, not a complete export of every unrelated search-result item. Do not treat search-result snippets as full-paper review.

## Linked results from those searches

| Returned title / authors / publication | Stable source | What the indexed result or accessible abstract establishes | Relationship to R-008 |
|---|---|---|---|
| **A Fast Time-Domain Algorithm for the Simulation of Switching Power Converters** — A. M. Luciano and A. G. M. Strollo (1990), *IEEE Transactions on Power Electronics*, 5(3), 363–370 | [University of Naples repository / abstract](https://www.iris.unina.it/handle/11588/456750); [DOI: 10.1109/63.56528](https://doi.org/10.1109/63.56528) | Uses **Chebyshev polynomial approximations to state-transition matrices** for switched power converters; a **buck converter** is a stated example, including topology-switching instants. | **Closest direct prior art**: the general concept of fast polynomial transition propagation *in this very application* is established. Our q=5 Taylor duty-polynomial versus exact compiled operator differs in basis, controller and benchmark specification, **not in the broad concept**. |
| **Computing the Matrix Exponential with an Optimized Taylor Polynomial Approximation** — P. Bader, S. Blanes, F. Casas (2019), *Mathematics* 7(12), 1174 | [Publisher abstract](https://www.mdpi.com/2227-7390/7/12/1174); [DOI: 10.3390/math7121174](https://doi.org/10.3390/math7121174) | Optimized evaluation of matrix-exponential Taylor polynomials can reduce matrix multiplication cost relative to other established matrix function methods. | Related numerical prior art showing Taylor evaluation is established; **not** the same PWM duty-kick benchmark. |
| **Computing the Action of the Matrix Exponential, with an Application to Exponential Integrators** — A. H. Al-Mohy and N. J. Higham (2011), *SIAM Journal on Scientific Computing* 33(2), 488–511 | [Publisher page and readable abstract](https://epubs.siam.org/doi/10.1137/100788860); [DOI: 10.1137/100788860](https://doi.org/10.1137/100788860) | Computes matrix-exponential action using truncated Taylor expansion and numerical error control; includes comparisons to specialist polynomial/Krylov methods. | Another relevant established operator-evaluation approach. It is **not evidence** of the particular R-008 performance number. |
| **Boosting the Computation of the Matrix Exponential** — J. Sastre, J. Ibáñez and E. Defez (2019), *Applied Mathematics and Computation* 340, 206–220 | [Publisher abstract](https://www.sciencedirect.com/science/article/abs/pii/S0096300318307318); [DOI: 10.1016/j.amc.2018.08.017](https://doi.org/10.1016/j.amc.2018.08.017) | Studies efficient Taylor algorithms for computing matrix exponentials, including optimized polynomial evaluation. | Additional independent evidence that faster polynomial methods are an established numerical design space. |

**Source-reading limitations:** The 1990 paper's **repository-indexed abstract** and bibliographic record were visible in search output; direct page open timed out. The 2019 *Mathematics* paper's **publisher-indexed abstract** was visible; direct open returned rate limiting. The 2011 SIAM **publisher abstract was directly opened and read**. No claim of reading the 1990 full article, no claim of exhaustive prior-art coverage.

## Nearest known predecessor and failure analysis

**Nearest:** Luciano & Strollo 1990, DOI 10.1109/63.56528. It already applied polynomial approximations of transition matrices to switched converters, **including the buck converter**. The broad research pitch “discover polynomial acceleration of converter switching dynamics” is **not novel on this evidence**. The search must have preceded P-080; it did not. The earlier branch instruction “consider prior art” was too weak: no compulsory executed-query record, pretest citation dossier or completion gate existed.

**What the R-008 experiment legitimately contributes:** a **REPORTED** numerical measurement of an order-five *duty-centered Taylor kick* against a **Numba JIT closed-form exact return**, with a declared 2,048-batch synthetic reuse workload and empirical accuracy. The measured ~1.209× cold-total ratio is a scoped engineering benchmark, **not** a newly invented polynomial acceleration method, proof of novelty, certification, or industrial demonstration. Different bases/workloads might be worth fair study **only** if a future pretest search finds a concrete untested comparison and the experiment is justified.

**Unresolved gap:** No evidence here that the fixed P-082 benchmark, exact duty-centered coefficients and JIT run conditions appeared verbatim in the literature; **that absence does not constitute scientific novelty or industrial importance**. Broader speedups and production use remain untested.

## How the corrected laboratory must behave next time

[PRIOR_ART.md](../PRIOR_ART.md) now requires actual external queries, date, results, primary source, closest predecessor, explicit unresolved delta and **pre-experiment Git commitment**. [PROOF_OF_DONE](../PROOF_OF_DONE.md) disallows `POD: YES` for new results lacking pretest evidence. This retrospective record is **not valid as a future P-card's pretest dossier**; subsequent experiments, including PWM extensions, must run and file their own targeted search first.
