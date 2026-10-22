# fd-dev

Native Rust development commands for Windows and Linux. This binary is not included
in application release packages.

```
cargo run -p fd-dev --release -- bench report RUN_DIR
# or, after building:
fd-dev bench report RUN_DIR
fd-dev check --jobs 2
fd-dev --help
```

`check [--jobs N]` runs the existing required checks in order: Cargo fmt, workspace
all-target check, strict Clippy (`-D warnings`), then workspace tests. Run it from the
repository root (or the workspace you intend to check). An explicit positive
`--jobs N` overrides `JOBS`; otherwise `JOBS` is used, with default2. Compiler jobs and
child `RUST_TEST_THREADS` are bounded by that count. The first failing stage stops
the workflow and returns the child's exit code, with its original output visible.
Use `fd-dev check --help` for usage. Linux CI uses the same workflow; Windows CI's
separate locked/test-skip/GPU policy remains intact.

`bench report` reads the captured BENC01 A/B/C JSONL, compile/GNU-time and comparison
logs, plus load/phases logs, and prints the same semantic `fd-benc01/1` JSON as the
retired `tools/benc01.py`. It does not render, invoke Python or require GNU tools.
Missing/malformed required inputs fail with a diagnostic and nonzero exit status.

**Compatibility limitation:** schema1 preserves the old reporter's hardcoded
historical path/settings/CPU metadata (960x540,100000iterations,12threads,Ryzen3900X).
These values are not discovered from a new run or the current machine. Use this
report for the matching historical benchmark configuration; do not interpret it as
a general benchmark runner or a new machine measurement. Benchmark execution and location workflows remain separate while TOOL-02 is in progress.

The captured compatibility fixture and its provenance are in `tests/fixtures/report`.
Native Linux/Windows CI drives the actual command and compares every JSON field
against the old Python output, and checks missing/malformed inputs.
