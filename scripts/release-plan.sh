#!/usr/bin/env bash
# DEC-32 release gate, shared by release.yml's plan job and ci.yml: prints `version=X`
# and `release=true|false` (GITHUB_OUTPUT lines). Fails when Cargo.toml's
# [workspace.package] version has no vX.Y.Z tag on origin and CHANGELOG.md has no
# notes for it, so a bump without notes is caught before it lands. NOTES=FILE keeps them.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\(.*\)"/\1/p}' Cargo.toml | head -1)
[ -n "$version" ] || { echo "no [workspace.package] version in Cargo.toml" >&2; exit 1; }
echo "version=$version"
if git ls-remote --exit-code --tags origin "refs/tags/v$version" >/dev/null; then
  echo "v$version is already released; nothing to do" >&2
  echo "release=false"
  exit 0
fi
bash scripts/release-notes.sh "$version" > "${NOTES:-/dev/null}"
echo "release=true"
