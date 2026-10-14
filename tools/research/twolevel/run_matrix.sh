#!/bin/bash
# usage: run_matrix.sh SCRATCH  (refs in SCRATCH/refs; writes SCRATCH/tr, SCRATCH/rec, SCRATCH/matrix.log)
S=$1; B=$(dirname $0)
mkdir -p $S/tr $S/rec
for w in ${WIDTHS:--10 -30 -100 -300}; do for loc in eye mis nuc; do
  v=$loc$w
  [ -f $S/tr/$v.t ] || $B/tile32 truth $S/refs/$v.ref 100000 8 $S/tr/$v.t | sed "s/^/$v /"
  for T in ${TILES:-4 8 16 64 2048}; do
    WLO=1 REC=$S/rec/$v.T$T $B/tile32 f32 $S/refs/$v.ref 100000 8 $T $S/tr/$v.t centre 1e-3 0 | head -2 | sed "s/^/$v /"
  done
done; done
