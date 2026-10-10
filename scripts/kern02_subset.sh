#!/usr/bin/env bash
# KERN-02 short DEC-17 gate: owner's 14 sampled v0 path frames.
set -euo pipefail
out=out/kern02-subset
threads=${THREADS:-4}
mkdir -p "$out/bla" "$out/zone"
python3 - "$out/path.txt" <<'PY'
from pathlib import Path
import sys

rows = [line for line in Path("bench/path-atlas-v0.txt").read_text().splitlines()
        if line.strip() and not line.lstrip().startswith("#")]
indices = (100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 400, 430, 480)
assert len(rows) == 750
Path(sys.argv[1]).write_text("# KERN-02 original frames " + " ".join(map(str, indices))
                             + "\n" + "\n".join(rows[i] for i in indices) + "\n")
PY
cargo build --release
bash tools/research/misiurewicz/make_zone.sh data/zones/v0.zone
fd=target/release/fd
flags=(--size 960x540 --iter 100000 --columns nu,de,normal
       --bla per-frame --runs 1 --threads "$threads")
"$fd" control "$out/path.txt" "${flags[@]}" -o "$out/bla" > "$out/bla.jsonl"
"$fd" control "$out/path.txt" "${flags[@]}" --zone data/zones/v0.zone \
    -o "$out/zone" > "$out/zone.jsonl"
compare_rc=0
"$fd" compare "$out/bla" "$out/zone" > "$out/compare.jsonl" || compare_rc=$?
python3 - "$out" <<'PY'
from pathlib import Path
import json
import sys

out = Path(sys.argv[1])
def frames(name):
    return {item["frame"]: item for line in (out / name).read_text().splitlines()
            if (item := json.loads(line)).get("record") == "frame"}

original = (100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 400, 430, 480)
b, z, c = (frames(name) for name in ("bla.jsonl", "zone.jsonl", "compare.jsonl"))
print("original  width        kernel                  bla_s  zone_s   speedup  compare")
for i, ix in enumerate(original):
    baseline, zone, score = b[i], z[i], c[i]
    bt, zt = baseline["runs"][0]["seconds"], zone["runs"][0]["seconds"]
    ok = score.get("ok", False)
    print(f"{ix:>8}  {zone['view']['width']:>11}  {zone['kernel'][:23]:<23}"
          f"  {bt:>5.3f}  {zt:>6.3f}  {bt/zt:>6.2f}x  {'ok' if ok else 'FAIL'}")
assert len(b) == len(z) == len(c) == len(original), "missing comparison frame"
PY
echo "Per-sample scoring: $out/compare.jsonl"
exit "$compare_rc"
