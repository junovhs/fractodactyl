# Research scripts (2026-10-07/08 session)

Python is research and oracle only (DEC-09). These are the exact scripts behind the
results in `docs/research/10-8-26/misiurewicz-frame-transfer.md` and the METHOD.md
results log. They are kept so the numbers can be reproduced; they are not production
code.

| Script | What it does | Run |
|---|---|---|
| `misiurewicz/returns_exit_tail.py` | **The key result.** For random pixels at given widths around the v0 target (period-764 nucleus near M(24,2)), it compares direct mpmath iteration against "k returns around the minibrot + escape of the exit point under z²+c0". It prints class mismatches and px displacement. | `python returns_exit_tail.py 1e-40 1e-46 2e-48` (about 1 min per width, 18 processes) |
| `misiurewicz/tanlei_one_return.py` | The earlier variant: one return, then the Tan Lei parameter-plane map (exact at 1e-30, fails deeper). | same |
| `misiurewicz/naive_z2_map.py` | The first, failed hypothesis ν(C+d) = ν(C+d+B²d²) + p. | same |
| `misiurewicz/compare_fds_displacement.py` | Compares two 480x270 `.fds` files (columns nu,de,normal): class equality and ν difference minus an offset, as px displacement. Used for the Misiurewicz similarity test. | `python compare_fds_displacement.py A.fds B.fds 2` |
| `misiurewicz/compare_fds_basic.py` | An earlier comparator (ν offset, de ratio, normal shift). | `python compare_fds_basic.py A.fds B.fds` |
| `misiurewicz/de_histogram.py` | The share of escaped pixels and iterations at ≥32/64/128/256 px from the set (this killed the analytic-patch idea). | `python de_histogram.py *.fds` |
| `explore/drive_explorer_cdp.mjs` | Drives `fd explore` in headless Chrome over DevTools (wheel, drag, preset key) and takes screenshots. | `node drive_explorer_cdp.mjs <chrome.exe> <profile dir> <out dir>` with `fd explore` running |
| `twist_zoom_path.py` | Writes an eased, rotating zoom path (the 10 s demo video). | `python twist_zoom_path.py RE IM > path.txt` |

The v0 target and the M(24,2) constants are embedded in the scripts. The `.fds`
comparators assume 480x270 and the column set noted. The Misiurewicz similarity renders
are reproduced in the research note.
