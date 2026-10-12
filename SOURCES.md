# External context and scope boundaries

This notebook was spawned from a Fractodactyl research thread, but **does not copy** that repository's renderer source or reports. Read original context on the main branch:

- [Fractodactyl method](https://github.com/junovhs/fractodactyl/blob/main/docs/spec/METHOD.md)
- [Fractodactyl research](https://github.com/junovhs/fractodactyl/tree/main/docs/research)
- [General-period cycle jumps](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-9-26/general-period-cycle-acceleration.md)
- [Factored return maps and six-agent synthesis](https://github.com/junovhs/fractodactyl/blob/main/docs/research/10-10-26/six-agent-final-takeaways-and-issue-roadmap.md)

Relevant PWM/hybrid prior art (this notebook does **not** claim novelty for state-space return maps or switching sensitivities):\n\n- [Sampled-data LPV voltage regulation for DC–DC buck converters (Electronics, 2022)](https://www.mdpi.com/2079-9292/11/19/3208) — sampled PWM duty from period-state measurements.\n- [Kong et al., Saltation Matrices: The Essential Tool for Linearizing Hybrid Dynamical Systems (IEEE, 2024)](https://doi.org/10.1109/jproc.2024.3440211) — event-time and sensitivity corrections in continuously state-triggered hybrids (beyond P-080's scheduled PWM model).\n\nPrior art to beat (not claims of novelty):

- [SIMPLIS periodic operating point](https://www.simplis.com/documentation/simplis/simplis_am/topics/periodic_operating_point.htm)
- [CAPD validated dynamics](https://capd.ii.uj.edu.pl/)
- Classical: Floquet theory; Koenigs/Sternberg linearization; nonlinear Poincaré maps; multiple shooting; direct polynomial composition; model-order reduction; time-parallel integration.

**Source caveat:** R-cards summarize prior assistant-reported probes, not verified archived raw results. Do not infer that the code is committed or that prior-art comparisons are exhaustive.
