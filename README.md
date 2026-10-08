# Fractadactyl

Fractadactyl is reset around one goal:

**Make genuine deep Mandelbrot rendering dramatically cheaper by compiling a known zoom path into a bounded, reusable mathematical atlas.**

The previous twin-minibrot swap prototype is intentionally absent from this branch. It remains in Git history on `ishoo/POC-02` (commit `790be5b`) for reference.

## Start here

0. **Start with `docs/spec/STATE.md`** (where we are, what to do next), then `ishoo_status`.
1. Read `SPEC.md`.
2. Read `docs/spec/ARCHITECTURE.md`.
3. Read `docs/spec/ISHOO-LEDGER.md`.
4. Read the research reports under `docs/research/` (one folder per date).
5. Read `docs/spec/METHOD.md` (how we work).

`fd` (crates/fd-cli) renders, compiles and plays genuine deep zooms; `viewer/explore.cmd` (Windows) or `viewer/explore.sh` (Linux; `--install` adds a desktop icon, `--stop` stops the renderer) opens a local browser explorer.
