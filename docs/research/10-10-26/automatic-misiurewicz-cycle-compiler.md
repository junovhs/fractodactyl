# Automatic Misiurewicz-cycle discovery — research implementation

Date: 2026-10-10 UTC. Branch: `research/auto-misiurewicz-discovery`.
**Research-only; not installed into the production `fd` renderer.**
The repository's Ishoo coordination hook was unavailable. Changes are isolated to a research branch.

## Implementation

- `tools/research/misiurewicz/auto_discover.py` takes camera centre, width, maximum iterations and frame dimensions. It builds a high-precision critical reference orbit at the exact decimal camera centre; searches for `z[q+p]≈z[q]`; Newton-refines primitive repelling cycles; computes cycle motion, multiplier and parameter derivative; and generates a parameter-dependent inverse Koenigs jet automatically. No period, point or multiplier is supplied at runtime.
- The compiler predicts the number of period returns, rejects unprofitable candidates, too-distant phase points, and weakly repelling cycles where derivative accuracy has not yet been demonstrated (currently `|lambda|<1.5`). It always retains direct-iteration fallback for pixels outside its local disk.
- `tools/research/misiurewicz/auto_cycle_bench/src/main.rs` is a separate native, multithreaded executor for compiled cycle constants and their parameter derivatives. It computes `nu`, pixel distance estimates and normals, preserving escaped vs unresolved sample kinds. The baseline remains the production `fd` executable, without modifications.
- `auto_discover.py` compares every sample with independent `fd control --bla per-frame --columns nu,de,normal --runs 1` output and samples worst derivative discrepancies with direct high-precision orbit-and-derivative iteration. The BLA cold time includes its in-frame preparation; the compiler time includes discovery and operator serialization. The native time includes process and output handling.

**Acceptance requirements:** exact three-way class agreement; every escaped pixel obeys `abs(nu_compiled - nu_fd) * ln(2) * de_fd / 2 <= 1e-3` output pixel; max relative DE difference < `1e-3`; max circular normal difference < `1e-3` radians. Full-frame checks are against `fd`, with high-precision independent spot diagnostics. The last two tolerances are additional conservative research gates, not implied by the 1e-3 pixel contract.

## Measurements

One native cold-frame run on GitHub Actions, 960x540 (518,400 samples), 20,000 iterations, 4 threads:

| Location | Discovered (q,p) | fd BLA cold | discovery+compile | native frame | total speedup | classes | max pixel displacement | max rel DE | max normal rad |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| c=i, width 1e-95 | (2,2) | 0.4120 s | 0.1009 s | 0.1656 s | 1.546x | 0 mismatches | 2.074e-10 | 3.035e-6 | 4.794e-5 |
| Unseen complex q=3,p=2, width 1e-95 | (3,2) | 0.3967 s | 0.1529 s | 0.1688 s | 1.233x | 0 mismatches | 7.689e-10 | 5.898e-4 | 7.652e-4 |

Both have zero pixels exceeding 1e-3 displacement. The unfamiliar q=3,p=2 frame eliminated ~119.9 million direct steps in aggregate. The tested location is defined by the approximately preperiodic camera centre:
`0.4196433776070805662759262823266433002120893730487961233893793197021016110409832128692177097141535071 + 0.606290729207199369259342197028023002949570668386421712214899686318868275281145662031327930379402341i`.

**Three separate cold-frame repeats at each 960x540 location**, all six passes (run [38033469892](https://github.com/junovhs/fractodactyl/actions/runs/38033469892)):

| Location | median total speedup | min | max | class mismatches | pixels beyond threshold |
|---|---:|---:|---:|---:|---:|
| c=i | 1.4346x | 1.4250x | 1.4816x | 0 | 0 |
| Unseen q=3,p=2 | 1.1677x | 1.1601x | 1.1717x | 0 | 0 |

The repeat measurements include discovery, compilation and rendering on each independent trial. These are small speedups; no extrapolation to other hardware or arbitrary locations is warranted.

## Correct decisions not to accelerate

- Generic camera `(-0.7453,0.1127)`, width `1e-3`: no convincing critical orbit recurrence at camera scale; decline.
- `c=-2`, width `1e-95`: return is at or outside the safe local phase domain; decline.
- `c=i`, width `1e-4`: detected period-two cycle, but the optimistic possible jump is too short to amortize the operator; decline without constructing the full Koenigs jet.
- Unseen complex `q=3,p=2`, width `1e-60`: a cycle is found but expected jump savings do not meet the heuristic cost; decline.
- Additional unseen location `-0.101096363845622161025785445738622565463805442826253483876931177660780840740470584274821219810516779 + 0.9562865108091415007710960577299774358098333365105291700343143215005246590657167325269784107873398072i`: detected `q=4,p=1`, `|lambda|≈1.33`. Earlier 480x270 native probe reached 1.187x nominal speedup with zero class mismatches and max pixel displacement ~3e-11, but maximum DE relative difference was 0.001448 (>0.001) and the gate **rejected** the operator. A direct high-precision diagnostic at the worst DE disagreement found fd DE error ~0.001217 and native error ~0.000229 against the exact orbit, meaning baseline discrepancy as well as candidate error was present. A conservative weak-repeller gate now declines this unvalidated case before compiling.

Relevant CI workflows:
- [Small probe](https://github.com/junovhs/fractodactyl/actions/workflows/auto-misiurewicz-probe.yml)
- [480x270 promotion](https://github.com/junovhs/fractodactyl/actions/workflows/auto-misiurewicz-promotion.yml)
- [960x540 full frames](https://github.com/junovhs/fractodactyl/actions/workflows/auto-misiurewicz-large.yml)
- [Three independent cold repeats](https://github.com/junovhs/fractodactyl/actions/workflows/auto-misiurewicz-repeat.yml)

## Limits and next gates

This is an automatically generated *finite-cycle* operator, not automatic minibrot renormalization or a universal dynamical compiler. The discovery pass currently computes its own high-precision reference orbit at the supplied camera centre. Candidate inference uses a finite (q <= 64, p <= 128) scan, a conservative local disk, a Taylor jet, and an empirical profitability heuristic. Its production use **cannot require rendering the BLA truth frame in order to choose the accelerator**; replace that research acceptance process with analytically validated error guards and a calibrated, benchmarked cost model. Validate mixed escaping/nonescaping pixels, off-centre Misiurewicz cameras, deeper iteration budgets, period larger than 128, and independent high-precision derivative oracles before integrating into `fd`. No main-branch code was modified.
