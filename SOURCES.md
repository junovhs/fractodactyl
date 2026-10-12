# External context and scope boundaries

This notebook was spawned from a Fractodactyl research thread, but **does not copy** that repository's renderer source or reports. Read original context on the main branch:

- [Fractodactyl method](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md)
- [Fractodactyl research](https://github.com/junovhs/fractodactyl/tree/main/docs/research)
- [General-period cycle jumps](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/general-period-cycle-acceleration.md)
- [Factored return maps and six-agent synthesis](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-10-26/six-agent-final-takeaways-and-issue-roadmap.md)

Relevant PWM/hybrid prior art (this notebook does **not** claim novelty for state-space return maps or switching sensitivities):

- [Sampled-data LPV voltage regulation for DC–DC buck converters (Electronics, 2022)](https://www.mdpi.com/2079-9292/11/19/3208) — sampled PWM duty from period-state measurements.
- [Kong et al., Saltation Matrices: The Essential Tool for Linearizing Hybrid Dynamical Systems (IEEE, 2024)](https://doi.org/10.1109/jproc.2024.3440211) — event-time and sensitivity corrections in continuously state-triggered hybrids (beyond P-080's scheduled PWM model).

**Retroactively discovered directly overlapping pre-1991 converter work:** [Luciano & Strollo (1990), “A Fast Time-Domain Algorithm for the Simulation of Switching Power Converters”](https://doi.org/10.1109/63.56528), [University of Naples indexed abstract](https://www.iris.unina.it/handle/11588/456750). Their Chebyshev polynomial approximation of converter transition matrices already included a buck converter. This was NOT checked before P-080/P-081/P-082; actual executed search queries/results and source-access limitations are preserved in [PA-PWM-RETRO](PRIOR_ART/PA-PWM-RETRO.md).

Related established numerical methods: [Bader, Blanes & Casas (2019), optimized matrix-exponential Taylor polynomials](https://doi.org/10.3390/math7121174); [Al-Mohy & Higham (2011), truncated Taylor algorithms for matrix-exponential action](https://doi.org/10.1137/100788860); [Sastre, Ibáñez & Defez (2019), boosting the matrix exponential](https://doi.org/10.1016/j.amc.2018.08.017).

**Mandatory forward-looking executed literature searches:** [PRIOR_ART](PRIOR_ART.md). Generic reference lists or these citations alone are **not** a pre-experiment search dossier.

Prior art to beat (not claims of novelty):

- [SIMPLIS periodic operating point](https://www.simplis.com/documentation/simplis/simplis_am/topics/periodic_operating_point.htm)
- [CAPD validated dynamics](https://capd.ii.uj.edu.pl/)
- Classical: Floquet theory; Koenigs/Sternberg linearization; nonlinear Poincaré maps; multiple shooting; direct polynomial composition; model-order reduction; time-parallel integration.

**Source caveat:** R-cards include both previous retrospective reports and newly reported sandbox probes. Executable/raw artifacts are not archived; these are **not independently reproduced**. Prior-art searches are non-exhaustive, and the 1990 converter connection was discovered only **after** the reported PWM measurements. Do not infer novelty or claim to have inspected a full inaccessible paper.

State-triggered event localization search (pretest [PA-P-083](PRIOR_ART/PA-P-083.md), result [R-009](RESULTS/R-009.md)): [Zhang, Yeddanapudi & Mosterman (2008)](https://doi.org/10.3182/20080706-5-KR-1001.01346), [Guglielmi & Hairer (2022)](https://doi.org/10.1007/s11075-021-01154-1); standard SciPy event root source https://github.com/scipy/scipy/blob/main/scipy/integrate/_ivp/ivp.py . No new algorithm claim.

## 2026-10-12 discovery triage — primary evidence

The following sources were located by **actual external searches on 2026-10-12**, prior to any proposed numerical work. Exact executed query examples and source-overlap dispositions are preserved in [FRONTIER.md](FRONTIER.md). These search-only literature eliminations do **not** constitute experiment preregistration.

- **R. P. Brent and J. F. Traub (1980)**, *On the Complexity of Composition and Generalized Composition of Power Series*, SIAM J. Comput. 9(1):54–66, [DOI 10.1137/0209004](https://doi.org/10.1137/0209004). Publisher abstract explicitly treats iteration of formal series by composition repeated squaring. Direct mechanism-level overlap with P-060's power-and-truncate idea; it does **not** establish a rigorous Mandelbrot repelling-cycle derivative/radius implementation.
- **Linda Petzold (1983)**, *Automatic Selection of Methods for Solving Stiff and Nonstiff Systems of Ordinary Differential Equations*, SIAM J. Sci. Stat. Comput. 4(1):136–148, [DOI 10.1137/0904010](https://doi.org/10.1137/0904010). Efficiency-based automatic switching among ODE method classes, with selection overhead assessed. Prior art for the generic mechanism in P-040, **not** for an identical Koenigs/composition operator menu or certified enclosure.
- **Stefan Schupp and Erika Ábrahám (2018)**, *Efficient Dynamic Error Reduction for Hybrid Systems Reachability Analysis*, TACAS, [DOI 10.1007/978-3-319-89963-3_17](https://doi.org/10.1007/978-3-319-89963-3_17). Open-access methods explicitly adapt accuracy/cost configurations along inconclusive paths and preserve/reuse earlier verified work. Overlaps certificate-aware computational admission and proposed reuse under hybrid-model refinement; specialized Fractodactyl trajectories are not their demonstration.
- **Parallel multiple shooting for the solution of initial value problems (1994)**, *Parallel Computing* 20(3):275–295, [DOI 10.1016/S0167-8191(06)80013-X](https://doi.org/10.1016/S0167-8191(06)80013-X). Primary abstract shows parallel multiple shooting for nonlinear IVPs given good approximate intermediate states—direct seed-dependency caution for P-051.
- **J. H. Chaudhry, D. Estep, S. Tavener, V. Carey and J. Sandelin (2016)**, *A Posteriori Error Analysis of Two-Stage Computation Methods with Application to Efficient Discretization and the Parareal Algorithm*, SIAM J. Numer. Anal. 54(5):2974–3002, [DOI 10.1137/16M1079014](https://doi.org/10.1137/16M1079014). A posteriori residual/adjoint error estimates applied to Parareal and other two-stage methods; **not** a rigorous MPFR enclosure of the target quadratic iteration.
- **Joris van der Hoeven (2025, author manuscript)**, *Certifying Trajectories of Dynamical Systems*, dated June 6, 2025, [author-hosted text](https://www.texmacs.org/joris/lagrange/lagrange.html). Exact-initial-condition high-precision analytic ODE trajectory, two-stage numeric computation then independently parallelizable global certification using local enclosures and dichotomic combination. Specifically observes the initial trajectory computation remains essentially sequential. Important near-overlap with P-051; the work concerns ODEs rather than the precise Mandelbrot recurrence.
- **Wayne B. Hayes and Kenneth R. Jackson**, *Rigorous Shadowing of Numerical Solutions of Ordinary Differential Equations by Containment*, [DOI 10.1137/S0036142901399100](https://doi.org/10.1137/S0036142901399100). Validates a true nearby trajectory; **do not confuse shadowing with certification of the original fixed initial condition**.

Scope caution: consulted publisher abstracts, an open full-text article and an author manuscript; paywalled full articles were not claimed as inspected. Nothing in this source list alone proves global novelty or a useful speedup.
