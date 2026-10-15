#!/bin/sh
# Zoom sequence for the flicker test: FRAMES frames at 0.5 decades/s and 30 fps
# (width x 10^(-1/60) per frame), 320x180 crop at the centre of each 1920x1080 frame.
# usage: run_seq.sh SCRATCH NAME RE IM WIDTH MAX_ITER FRAMES
set -e
S=$1; N=$2; RE=$3; IM=$4; W=$5; IT=$6; F=$7
B=$(dirname "$0")/target/release
mkdir -p "$S/seq-$N"
[ -f "$S/seq-$N/ref" ] || python3 "$(dirname "$0")/refgen.py" "$RE" "$IM" "$W" "$IT" "$S/seq-$N/ref"
k=0
while [ $k -lt $F ]; do
  wk=$(python3 -c "print(repr($W * 10 ** (-$k / 60)))")
  f=$(printf "%s/seq-%s/f%03d.dmp" "$S" "$N" $k)
  [ -f "$f" ] || "$B/render" "$S/seq-$N/ref" "$wk" 320 180 800 450 "$IT" "$f" > /dev/null
  k=$((k + 1))
done
"$B/score" --seq "$S"/seq-"$N"/f*.dmp
