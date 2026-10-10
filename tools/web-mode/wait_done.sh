#!/bin/bash
# Web mode: block until ChatGPT signals a finished turn by pushing a new web-mode/ID.done on
# gpt/ID, then print that file (summary + the commands to run). Run it in the background.
# Checks GitHub every 30 s; never reads chatgpt.com. See docs/spec/WEB-MODE.md.
#   wait_done.sh ID [MINUTES=120]
id=$1; mins=${2:-120}; f="web-mode/$id.done"
blob() {  # blob id of the done file on origin/gpt/ID, or empty
  git fetch -q origin "gpt/$id" 2>/dev/null && git rev-parse -q --verify "FETCH_HEAD:$f" 2>/dev/null
}
h0=$(git ls-remote origin "refs/heads/gpt/$id" | cut -f1); b0=$(blob)
for _ in $(seq 1 $((mins * 2))); do
  sleep 30
  h=$(git ls-remote origin "refs/heads/gpt/$id" | cut -f1)
  [ -n "$h" ] && [ "$h" != "$h0" ] || continue
  h0=$h; b=$(blob)
  if [ -n "$b" ] && [ "$b" != "$b0" ]; then
    echo "done: gpt/$id at $(git rev-parse --short FETCH_HEAD)"; git show "FETCH_HEAD:$f"; exit 0
  fi
done
echo "no done file in $mins min (last head ${h0:-none})"; exit 1
