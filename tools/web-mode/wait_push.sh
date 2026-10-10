#!/bin/bash
# Web mode: block until gpt/ID moves on origin, then wait SETTLE seconds more (ChatGPT
# often pushes several commits), and print the head. Run it in the background.
#   wait_push.sh ID [MINUTES=120] [SETTLE=600]
id=$1; mins=${2:-120}; settle=${3:-600}
h0=$(git ls-remote origin "refs/heads/gpt/$id" | cut -f1)
for _ in $(seq 1 "$mins"); do
  h=$(git ls-remote origin "refs/heads/gpt/$id" | cut -f1)
  if [ -n "$h" ] && [ "$h" != "$h0" ]; then
    sleep "$settle"; echo "pushed $(git ls-remote origin "refs/heads/gpt/$id" | cut -f1)"; exit 0
  fi
  sleep 60
done
echo "no push in $mins min"; exit 1
