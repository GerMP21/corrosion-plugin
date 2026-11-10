#!/usr/bin/env bash
# Run the host-facing validators on one platform's bundles
# (`extend-ci-windows-linux-macos`).
#
#   scripts/validate-plugins.sh [linux|windows|macos] [dir]
#
# The platform defaults to the one this runs on; `dir` to
# target/bundled/<platform> (built first with scripts/bundle.sh if missing).
# pluginval checks the VST3 (strictness 5, no GUI tests) and clap-validator
# the CLAP. A missing validator is skipped with a message, unless
# CORROSION_REQUIRE_VALIDATORS=1 (the CI), which makes it an error.
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${PROJECT_DIR}"

case "$(uname -s)" in
    Linux) HOST=linux ;;
    Darwin) HOST=macos ;;
    MINGW* | MSYS* | CYGWIN*) HOST=windows ;;
    *) HOST=linux ;;
esac
PLATFORM="${1:-$HOST}"
DIR="${2:-target/bundled/${PLATFORM}}"
REQUIRE="${CORROSION_REQUIRE_VALIDATORS:-0}"

if [ ! -e "${DIR}/Corrosion.vst3" ] || [ ! -e "${DIR}/Corrosion.clap" ]; then
    echo "==> Bundles missing in ${DIR}, building them first"
    scripts/bundle.sh "${PLATFORM}" release
fi

missing() {
    if [ "$REQUIRE" = 1 ]; then
        echo "error: $1 not found (CORROSION_REQUIRE_VALIDATORS=1)" >&2
        exit 1
    fi
    echo "$1 not found, skipping"
}

if command -v pluginval >/dev/null 2>&1; then
    echo "==> pluginval (${PLATFORM})"
    pluginval --strictness-level 5 --skip-gui-tests --validate "${DIR}/Corrosion.vst3"
else
    missing pluginval
fi

if command -v clap-validator >/dev/null 2>&1; then
    echo "==> clap-validator (${PLATFORM})"
    clap-validator validate "${DIR}/Corrosion.clap"
else
    missing clap-validator
fi
