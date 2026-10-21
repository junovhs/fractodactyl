#!/usr/bin/env bash
# Actual CLI proof: generated zones versus preserved pre-change zones and frozen frames.
# FD must name a built fd. OUT must be outside the checkout.
set -euo pipefail
fd=${FD:?set FD to the built fd binary}
out=${OUT:?set OUT to an external evidence directory}
mkdir -p "$out/generated" "$out/old-core" "$out/old-full" "$out/new-core" "$out/new-full"
if rg -n 'koenigs_bench|make_zone.sh' README.md docs/spec/STATE.md scripts/kern02_*.sh .github; then
    echo 'live instruction still points at a retired producer' >&2
    exit 1
fi
test ! -e .github/workflows/koenigs-bench.yml
# Audit all requested surfaces; remaining hits are immutable fixture provenance,
# explicitly marked historical records, or this assertion itself.
while IFS= read -r path; do
    case "$path" in
        docs/spec/METHOD.md|scripts/check/CLEA-02.sh|crates/fd-cli/tests/fixtures/zones/*.zone) ;;
        docs/spec/WEB-MODE.md|docs/research/10-10-26/gpu-precision-work-queues/README.md|docs/research/10-10-26/gpu-precision-work-queues/REPORT.md|docs/research/10-9-26/HANDOFF-mid-band-jump.md|docs/research/10-8-26/misiurewicz-frame-transfer.md)
            rg -q 'Historical research record.*frozen prototypes|Historical research record' "$path" ;;
        *) echo "unclassified retired-producer reference: $path" >&2; exit 1 ;;
    esac
done < <(rg -l 'koenigs_bench|make_zone.sh' -- docs crates scripts .github README.md)
# Exact saved input; never parse an absolute coordinate through a machine float.
read -r re im _ < <(awk '!/^#/ && NF {print; exit}' bench/path-atlas-v0.txt)
"$fd" zone "$re" "$im" -o "$out/generated/v0.zone" > "$out/generate.json"
# The core fixture deliberately omits tail patches, mid-band operator and reference.
awk '!/^(patch_|node |leaf |mid_|ref )/' "$out/generated/v0.zone" > "$out/generated/v0-core.zone"
for width in 1e-35 1e-38 1e-40 1e-43 1e-46 2e-48; do
    for kind in old-core old-full new-core new-full; do
        case "$kind" in
            old-core) zone="$out/before/v0-core.zone" ;;
            old-full) zone="$out/before/v0-full.zone" ;;
            new-core) zone="$out/generated/v0-core.zone" ;;
            new-full) zone="$out/generated/v0.zone" ;;
        esac
        mkdir -p "$out/$kind/$width"
        "$fd" render --re "$re" --im "$im" --width "$width" --size 480x270 \
            --threads "${ISHOO_CPU_ALLOCATION:-1}" --iter 100000 --columns nu,de,normal --zone "$zone" \
            -o "$out/$kind/$width/frame-00000.fds" > "$out/$kind/$width.stdout" 2> "$out/$kind/$width.stderr"
    done
    "$fd" compare "$out/old-core/$width" "$out/new-core/$width" > "$out/core-$width.jsonl"
    "$fd" compare "$out/old-full/$width" "$out/new-full/$width" > "$out/full-$width.jsonl"
done
echo 'CLEA-02: six PROB-08 frames, old/new core and full zones: 0 comparison failures'
