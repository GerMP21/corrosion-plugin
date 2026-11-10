#!/usr/bin/env bash
# Linux VST3 and CLAP bundles: a wrapper around scripts/bundle.sh (output in
# target/bundled/linux/).
set -euo pipefail
exec "$(cd "$(dirname "$0")" && pwd)/scripts/bundle.sh" linux "${1:-release}"
