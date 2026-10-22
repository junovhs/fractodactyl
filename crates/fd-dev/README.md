# fd-dev

Native Rust development commands for Windows and Linux. This binary is not included
in application release packages.

```
cargo run -p fd-dev --release -- bench report RUN_DIR
# or, after building:
fd-dev bench report RUN_DIR
fd-dev --help
```

`bench report` reads the captured BENC01 A/B/C JSONL, compile/GNU-time and comparison
logs, plus load/phases logs, and prints the same semantic `fd-benc01/1` JSON as the
retired `tools/benc01.py`. It does not render, invoke Python or require GNU tools.
Missing/malformed required inputs fail with a diagnostic and nonzero exit status.

**Compatibility limitation:** schema1 preserves the old reporter's hardcoded
historical path/settings/CPU metadata (960x540,100000iterations,12threads,Ryzen3900X).
These values are not discovered from a new run or the current machine. Use this
report for the matching historical benchmark configuration; do not interpret it as
a general benchmark runner or a new machine measurement. Rendering workflows are
still separate while TOOL-02 is in progress.

The captured compatibility fixture and its provenance are in `tests/fixtures/report`.
Native Linux/Windows CI drives the actual command and compares every JSON field
against the old Python output, and checks missing/malformed inputs.
