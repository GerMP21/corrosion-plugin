#!/usr/bin/env bash
# Windows VST3 and CLAP bundles with the mingw-w64 GNU toolchain of the
# maintainer's machine: a wrapper around scripts/bundle.sh (output in
# target/bundled/windows/). The CI ships the MSVC build instead.
set -euo pipefail
export CORROSION_WINDOWS_TARGET="${CORROSION_WINDOWS_TARGET:-x86_64-pc-windows-gnu}"
exec "$(cd "$(dirname "$0")" && pwd)/scripts/bundle.sh" windows "${1:-release}"
