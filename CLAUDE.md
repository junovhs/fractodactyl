<!-- ishoo:begin -->
This repository uses Ishoo for coordination. Start with `ishoo_status`, take work with `ishoo_claim`, and finish with `ishoo_land` (say what changed and which tests you ran). Pick work by what moves the active plan's outcome most. If main is reported broken, fix it before building on it.
<!-- ishoo:end -->


## How work is done here: fail fast

- Test ideas with small probes that run in under 30 seconds (builds excepted). No 10-minute tests.
- A losing result gets a few lines in an issue comment (what was tried, the numbers) and the idea is dropped or redirected. No proof suites, evidence packs, research write-ups or follow-up phases for a losing path.
- Heavy proof (full-frame gates, multi-device runs, film checks) only after a fast probe shows a real win.
- Benchmark and test cameras come from the real target film frames (`docs/2026-10-20/series-frames/results/frames.txt`), never from personal saved places.
