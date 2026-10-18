#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm ci --no-audit --no-fund
node --test ../scripts/refresh-site.test.mjs
./check-download.sh
npm run build
npm run check:docs
./check-nav.sh
