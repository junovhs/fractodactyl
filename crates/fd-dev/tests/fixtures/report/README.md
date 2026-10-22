# Fresh tiny compatibility capture, not BENC01 performance evidence

These are **real newly captured inputs**, not reconstructed historical data and not
a rerun of the published BENC01 benchmark. Captured on Linux using the current fd
code after 805f2262 (the CLI was unchanged by that truth-pack commit), at 8x8,
32 iterations, 1 worker. 76 frames exercise both full and partial 75-frame report bins.
Actual A/plain, B/per-frameBLA, atlas compile/play and AB/AC/BC comparisons ran;
GNU-time, observed system load and phase timestamps were captured directly.
`capture.json` records the commands and `path.txt` the exact generated path.

`expected.json` is the unchanged legacy Python reporter's output on these inputs.
Its historical hardcoded960x540/100000iteration/12thread/Ryzen3900X settings are
preserved **solely to test compatibility**. They do not describe this tiny run.
Do not cite its speedups, memory, settings or timestamps as benchmark evidence.
The published `bench/benc-01-results.json` is untouched.

`provenance.json` records the legacy reporter hash/source commit and input hashes.
Recreate only as a new fixture; timing values will differ:

```
git show 805f2262:tools/benc01.py > /external/benc01.py
python3 tools/research/tool03/capture.py --fd /external/fd --legacy-reporter /external/benc01.py --out /external/new-capture
```

The Rust integration test invokes the actual `fd-dev bench report` binary, compares
all JSON fields semantically (including exact binary64 numeric equality), and checks
clean failures. The same tests run on native Linux and Windows with Rust 1.90.
