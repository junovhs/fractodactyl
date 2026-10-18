#!/usr/bin/env bash
# DOCS-04 check: the entry docs no longer describe removed tools, flags or UI.
# Exits 1 and prints each stale line found, 0 when none remain.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" || exit 2
fail=0
stale() { # FILE PATTERN WHY
  if grep -nF -- "$2" "$1" >/dev/null; then
    echo "stale ($3): $1"; grep -nF -- "$2" "$1" | sed 's/^/    /'; fail=1
  fi
}
stale README.md "# once: the v0 zone (needs python3" "zones are built by fd zone -o (PROB-12)"
stale docs/spec/STATE.md "until PROB-12 lands" "PROB-12 landed"
stale docs/spec/STATE.md 'Add `--gpu-kernel mixed` to render zone keyframes' "zone frames run on the wgpu kernel by default (CLEA-06)"
stale docs/spec/STATE.md "GPU-21 (zone output stays on the GPU" "GPU-21 is done"
stale Cargo.toml "raw Vulkan" "the ash/Vulkan stack is gone (SHAD-11)"
stale viewer/README-v2.md "Fossils" "V2 toolbar is Explore, Places, Saved, Snapshot"
stale viewer/README-v2.md "Nest:" "V2 toolbar is Explore, Places, Saved, Snapshot"
stale viewer/README-v2.md "- Presets:" "the sidebar panel is Looks"
stale docs/spec/EXPLORE.md "1/8, 1/4 and 1/2 resolution" "navigation streams tiles (VIEW-08)"
stale crates/fd-cli/src/main.rs "fd zone RE IM [--max-iter N]   (zone structure report" "fd zone -o writes zones (PROB-12)"
[ "$fail" = 0 ] && echo "ok: no stale markers in the entry docs"
exit "$fail"
