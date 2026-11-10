#!/usr/bin/env bash
# Check the layout of the bundles `scripts/bundle.sh` produced for one
# platform (`extend-ci-windows-linux-macos`).
#
#   scripts/check_bundle.sh <linux|windows|macos> [dir]
#
# `dir` defaults to target/bundled/<platform>. Checks that each format sits
# where hosts look for it, that the binary is the platform's kind of shared
# library, and on macOS that it holds arm64 and x86_64 code, has an
# Info.plist and verifies as code-signed. Exits non-zero on the first
# failure, naming it.
set -euo pipefail

PLATFORM="${1:?usage: check_bundle.sh <linux|windows|macos> [dir]}"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIR="${2:-${PROJECT_DIR}/target/bundled/${PLATFORM}}"
NAME="Corrosion"

fail() {
    echo "check_bundle: ${PLATFORM}: $*" >&2
    exit 1
}

need_file() {
    [ -f "$1" ] || fail "missing $1"
}

# `file` describes the binary; match the platform's shared-library kind.
need_kind() {
    local path="$1" pattern="$2" what
    what="$(file -b "$path")"
    echo "$what" | grep -Eq "$pattern" || fail "$path is not $pattern: $what"
}

case "$PLATFORM" in
    linux)
        need_file "${DIR}/${NAME}.vst3/Contents/x86_64-linux/${NAME}.so"
        need_kind "${DIR}/${NAME}.vst3/Contents/x86_64-linux/${NAME}.so" "ELF 64-bit .*shared object"
        need_file "${DIR}/${NAME}.clap"
        need_kind "${DIR}/${NAME}.clap" "ELF 64-bit .*shared object"
        ;;
    windows)
        need_file "${DIR}/${NAME}.vst3/Contents/x86_64-win/${NAME}.vst3"
        need_kind "${DIR}/${NAME}.vst3/Contents/x86_64-win/${NAME}.vst3" "PE32\+ executable.*DLL"
        need_file "${DIR}/${NAME}.clap"
        need_kind "${DIR}/${NAME}.clap" "PE32\+ executable.*DLL"
        ;;
    macos)
        for bundle in "${NAME}.vst3" "${NAME}.clap"; do
            binary="${DIR}/${bundle}/Contents/MacOS/${NAME}"
            need_file "$binary"
            need_file "${DIR}/${bundle}/Contents/Info.plist"
            need_file "${DIR}/${bundle}/Contents/PkgInfo"
            need_kind "$binary" "Mach-O universal binary"
            archs="$(lipo -archs "$binary")"
            for arch in arm64 x86_64; do
                echo " $archs " | grep -q " $arch " || fail "$binary lacks $arch (has: $archs)"
            done
            codesign --verify --strict "${DIR}/${bundle}" || fail "${bundle} is not validly signed"
        done
        ;;
    *)
        fail "unknown platform (linux, windows or macos)"
        ;;
esac

echo "check_bundle: ${PLATFORM}: ok (${DIR})"
