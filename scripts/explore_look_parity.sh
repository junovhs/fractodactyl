#!/usr/bin/env bash
# EXPL-05 parity: fd explore look mode (WebGL2 in headless Chrome) vs fd shade on the same samples.
# Usage: scripts/explore_look_parity.sh FD OUTDIR W H WIDTH PRESET [ITER]
set -u
FD=$1; S=$2; W=$3; H=$4; WIDTH=$5; PRESET=$6; ITER=${7:-100000}
re='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
im='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
mkdir -p $S; rm -f $S/cap.rgba $S/cap.rgba.fds
$FD explore --port 8799 --threads 12 --capture $S/cap.rgba > $S/explore-test.log 2>&1 & EP=$!
sleep 1
URL="http://127.0.0.1:8799/?test=1&W=$W&H=$H&re=$re&im=$im&w=$WIDTH&iter=$ITER&preset=$PRESET"
google-chrome --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --disable-gpu-sandbox --no-first-run \
  --user-data-dir=$S/chrome-prof --remote-debugging-port=9333 "$URL" > $S/chrome.log 2>&1 & CP=$!
for i in $(seq 120); do [ -s $S/cap.rgba ] && break; sleep 1; done
sleep 0.5; kill $CP $EP 2>/dev/null; wait 2>/dev/null
[ -s $S/cap.rgba ] || { echo "no capture"; exit 1; }
rm -rf $S/capin $S/capshade; mkdir -p $S/capin; cp $S/cap.rgba.fds $S/capin/frame-00000.fds
$FD shade $S/capin --preset $PRESET --aa on --unresolved interior -o $S/capshade > /dev/null
python3 - "$S" "$W" "$H" <<'PY'
import sys, numpy as np
from PIL import Image
S, W, H = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
g = np.frombuffer(open(f'{S}/cap.rgba', 'rb').read(), np.uint8).reshape(H, W, 4)[::-1, :, :3].astype(float)
r = np.asarray(Image.open(f'{S}/capshade/frame-00000.studio.png')).astype(float)
d = np.abs(g - r)
print(f'GPU vs fd shade: mean abs diff {d.mean():.3f}  p99 {np.percentile(d, 99)}  p99.9 {np.percentile(d, 99.9)}  max {d.max()}  pixels>8: {(d.max(axis=2) > 8).sum()} of {W*H}')
Image.fromarray(g.astype(np.uint8)).save(f'{S}/cap-gpu.png')
PY
