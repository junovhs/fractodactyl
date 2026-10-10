#!/usr/bin/env bash
# KERN-02 DEC-17 whole-path gate and 1080p60 keyframe timing.
# Usage: THREADS=24 bash scripts/kern02_gate.sh [control|film|all]
set -euo pipefail
mode=${1:-all}
if [[ "$mode" != control && "$mode" != film && "$mode" != all ]]; then
    echo "usage: bash scripts/kern02_gate.sh [control|film|all]" >&2
    exit 2
fi
out=out/kern02
mkdir -p "$out"
threads=${THREADS:-4}
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release
bash tools/research/misiurewicz/make_zone.sh data/zones/v0.zone
fd=target/release/fd

gate_fail=0
if [[ "$mode" == control || "$mode" == all ]]; then
    mkdir -p "$out/bla" "$out/zone"
    "$fd" control bench/path-atlas-v0.txt --size 960x540 --iter 100000 \
        --threads "$threads" --columns nu,de,normal --bla per-frame \
        --runs 1 -o "$out/bla" > "$out/bla.jsonl"
    "$fd" control bench/path-atlas-v0.txt --size 960x540 --iter 100000 \
        --threads "$threads" --columns nu,de,normal --bla per-frame \
        --zone data/zones/v0.zone --runs 1 -o "$out/zone" > "$out/zone.jsonl"
    compare_ok=0
    "$fd" compare "$out/bla" "$out/zone" > "$out/compare.jsonl" || compare_ok=$?
    # fd compare is intentionally strict: FIX-04 and the one known old deep
    # mismatch make it exit 1 even when KERN-02 passes. Preserve its JSONL
    # unmodified; the scorer below implements the explicitly scoped waiver.
    score_ok=0
    python3 - "$out" <<'PY' || score_ok=$?
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
def records(name):
    rows = [json.loads(line) for line in (out / name).read_text().splitlines()]
    frames = [r for r in rows if r.get("record") == "frame"]
    if len(frames) != len({r["frame"] for r in frames}):
        raise ValueError(f"{name}: duplicate frame numbers")
    totals = [r for r in rows if r.get("record") == "totals"]
    return {r["frame"]: r for r in frames}, totals

b, bt = records("bla.jsonl")
z, zt = records("zone.jsonl")
c, ct = records("compare.jsonl")
expected = set(range(750))
if set(b) != expected or set(z) != expected or set(c) != expected:
    raise ValueError(
        f"missing/extra frames: bla={len(b)}, zone={len(z)}, compare={len(c)}; "
        f"expected exactly frames 0..749"
    )
if len(ct) != 1 or ct[0].get("frames") != 750 or ct[0].get("errors") != 0:
    raise ValueError("fd compare totals missing, partial or contain read errors")

fix04 = {}
main_baseline = {}
bad = {}
for i in sorted(expected):
    rec = c[i]
    kinds = rec["kinds"]
    deep = z[i]["kernel"].startswith("zone-koenigs/")
    # FIX-04: reference BLA exhausts its 100000-iteration budget, whereas
    # the existing deep zone detects an interior point. Not a KERN-02 error.
    fix04[i] = kinds["unresolved_to_interior"] if deep else 0
    # Only one independently reproduced old discrepancy is exempted:
    # main d051685 also differs on frame 748, escaped -> unresolved (1 px).
    # Any additional occurrence, even on frame 748, fails the gate.
    main_baseline[i] = min(kinds["escaped_to_unresolved"], 1) if deep and i == 748 else 0
    other_classes = rec["class_mismatches"] - fix04[i] - main_baseline[i]
    bad[i] = (
        "error" in rec
        or sum(kinds.values()) != rec["class_mismatches"]
        or other_classes != 0
        or rec["non_finite"] != 0
        or rec["nu"]["over_px"] != 0
        or rec["de"]["over_tol"] != 0
        or rec["normal"]["over_tol"] != 0
        or rec["width_rel"] > 1e-15
    )

print("KERN-02 full-path gate (s, wall times for same single-run frames)")
for group, members in (
    ("mid", [i for i, x in z.items() if x["kernel"].startswith("zone-mid-")]),
    ("deep", [i for i, x in z.items() if x["kernel"].startswith("zone-koenigs/")]),
    ("outside", [i for i, x in z.items() if not x["zone"]["used"]]),
    ("all", sorted(expected)),
):
    bt_seconds = sum(b[i]["runs"][0]["seconds"] for i in members)
    zone_seconds = sum(z[i]["runs"][0]["seconds"] for i in members)
    failures = [i for i in members if bad[i]]
    fix04_count = sum(fix04[i] for i in members)
    print(
        f"{group}: frames={len(members)} bla={bt_seconds:.4f} "
        f"zone={zone_seconds:.4f} speedup={bt_seconds/zone_seconds if zone_seconds else 0:.3f}x "
        f"failed={len(failures)} fix04_unresolved_to_interior={fix04_count}"
    )

fix04_frames = [i for i in sorted(expected) if fix04[i]]
known_frames = {i: main_baseline[i] for i in sorted(expected) if main_baseline[i]}
rejected = [i for i in sorted(expected) if bad[i]]
print(
    f"FIX-04 baseline-limited classification (not KERN-02): "
    f"pixels={sum(fix04.values())} frames={len(fix04_frames)} "
    f"frame_indices={fix04_frames}"
)
print(
    f"Known pre-existing main d051685 mismatch (not KERN-02): "
    f"escaped_to_unresolved={known_frames}"
)
print(f"compared={len(c)} expected={len(b)} rejected_frames={rejected}")
if rejected:
    for i in rejected:
        print(
            f"FAIL frame={i} kinds={c[i]['kinds']} "
            f"nu_over={c[i]['nu']['over_px']} de_over={c[i]['de']['over_tol']} "
            f"normal_over={c[i]['normal']['over_tol']} "
            f"non_finite={c[i]['non_finite']}"
        )
if rejected:
    sys.exit(1)
PY
    echo "fd compare exit: $compare_ok (raw strict class agreement)"
    if (( compare_ok > 1 || score_ok != 0 )); then
        echo "KERN-02 correctness gate failed; inspect $out/compare.jsonl" >&2
        gate_fail=1
    fi
fi

if [[ "$mode" == film || "$mode" == all ]]; then
    re=-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502
    im=0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922
    args=("$re" "$im" --from 4 --to 2e-49 --seconds 25 --fps 60
          --size 1920x1080 --iter 100000 --threads "$threads" --keyframes 2)
    "$fd" film "${args[@]}" --zone data/zones/v0.zone \
        --mp4 "$out/v0-1080p60-zone.mp4" > "$out/film-zone.jsonl"
    "$fd" film "${args[@]}" --frames 1200..1250 \
        --mp4 "$out/v0-deep-bla.mp4" > "$out/film-deep-bla.jsonl"
    "$fd" film "${args[@]}" --frames 1200..1250 --zone data/zones/v0.zone \
        --mp4 "$out/v0-deep-zone.mp4" > "$out/film-deep-zone.jsonl"
    echo "Keyframe timings: $out/film-zone.jsonl, film-deep-bla.jsonl, film-deep-zone.jsonl"
fi

exit "$gate_fail"
