#!/usr/bin/env bash
# Archived (kept for old habits): use scripts/bundle.sh linux instead.
# Linux VST3 and CLAP bundles: a wrapper around scripts/bundle.sh (output in
# target/bundled/linux/).
set -euo pipefail
exec "$(cd "$(dirname "$0")/.." && pwd)/bundle.sh" linux "${1:-release}"
