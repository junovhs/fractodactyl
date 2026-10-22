<!-- ishoo:begin -->
This repository is managed by Ishoo. Before handling the first user request, call the `ishoo_brief` MCP tool, and drive all issue, plan, and decision work through the `ishoo_*` MCP tools, never the Ishoo command-line interface. If those tools fail, retry: a dropped connection usually recovers on the next call. If Ishoo stays unavailable, never touch `main`: commit your work on a branch and push nothing (in a cloud session where Ishoo is not installed, push only that branch), then tell the user Ishoo must be enabled.
<!-- ishoo:end -->


## How work is done here: fail fast

- Test ideas with small probes that run in under 30 seconds (builds excepted). No 10-minute tests.
- A losing result gets a few lines in an issue comment (what was tried, the numbers) and the idea is dropped or redirected. No proof suites, evidence packs, research write-ups or follow-up phases for a losing path.
- Heavy proof (full-frame gates, multi-device runs, film checks) only after a fast probe shows a real win.
- Benchmark and test cameras come from the real target film frames (`docs/2026-10-20/series-frames/results/frames.txt`), never from personal saved places.
