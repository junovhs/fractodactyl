# Evidence execution guard (research-only)

Run from a checkout of the lab branch; the script also works in isolated scratch.
It uses only Python standard-library modules.

```sh
python scripts/research_run.py run --source probe.py --config probe.json --out EVIDENCE/R-NNN -- python probe.py
python scripts/research_run.py verify EVIDENCE/R-NNN/<actual-run-id>/manifest.json
python scripts/test_research_run.py
```

A probe must write its output files to `os.environ["DTL_RUN_DIR"]`. The runner
captures stdout/stderr, source/input/config snapshots, command, start/end UTC,
exit status and SHA-256 for each file. Every launch gets a new run ID. Changed
inputs, config or command change the fingerprint; source mutations fail the run.
The `verify` action fails closed on missing/changed/unlisted archived files.
If a separate program implements checkpoints, it must include `run_id` and
`fingerprint` in each checkpoint and call `check-resume MANIFEST CHECKPOINT`;
row-count-only resume is prohibited.

The runner does **not** validate mathematics, detect unlisted external inputs,
prove a floating-point error bound, sandbox the command, or automatically
enforce itself on agents. Declare every source, config, data input and
environment-affecting setting. Put seeds, compiler options and precision in
config, and use explicit shell commands for compilation. Verify that source
content was genuinely read from the stated snapshot for strict reproducibility.

Commit only small, meaningful text/source/config/raw evidence to the lab branch.
Large outputs require a stable, actually retrieved archive with recorded hashes.
No compiled binaries, dependencies or renderer code belong on this branch.
