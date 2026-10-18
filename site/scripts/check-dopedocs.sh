#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
npm ci --no-audit --no-fund
./check-download.sh
npm run build
npm run check:docs
./check-nav.sh
