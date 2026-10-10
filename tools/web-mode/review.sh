#!/bin/bash
# Web mode: merge origin/gpt/ID into the issue's Ishoo worktree (after ishoo_start ID), then
# cargo test + clippy. Prints new commits, diffstat, and only failing lines.
#   review.sh ID
id=$1; repo=$(git rev-parse --path-format=absolute --git-common-dir | xargs dirname)
cd "$repo/.ishoo/worktrees/$id" || { echo "no worktree: run ishoo_start $id first"; exit 1; }
git fetch -q origin "gpt/$id" && git log --oneline HEAD..FETCH_HEAD && git diff --stat HEAD...FETCH_HEAD | tail -8
git merge -q --ff-only FETCH_HEAD || git merge -q --no-edit FETCH_HEAD || exit 2
git rm -rq --ignore-unmatch web-mode   # ChatGPT's done files never land on main
export CARGO_TARGET_DIR=$repo/target/ishoo-worktrees/shared CARGO_BUILD_JOBS=${JOBS:-4} RUST_TEST_THREADS=${JOBS:-4}
cargo test --workspace 2>&1 | grep -E "FAILED|panicked|^error" | head; echo "== test failures above (none = pass)"
cargo clippy --workspace --all-targets 2>&1 | grep -E "^(warning|error)" -A3 | grep -- "-->" | head; echo "== clippy findings above (none = clean)"
