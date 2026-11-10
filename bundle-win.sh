#!/usr/bin/env bash
set -euo pipefail

PLUGIN_NAME="Corrosion"
TARGET="x86_64-pc-windows-gnu"
PROFILE="${1:-release}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
TARGET_DIR="${PROJECT_DIR}/../corrosion-target"

echo "Building ${PLUGIN_NAME} for ${TARGET} (${PROFILE})..."
cargo build --target "${TARGET}" --"${PROFILE}" --lib

BUNDLE_DIR="${PROJECT_DIR}/target/bundled-win/${PLUGIN_NAME}.vst3"
rm -rf "${BUNDLE_DIR}"
mkdir -p "${BUNDLE_DIR}/Contents/x86_64-win"

DLL_PATH="${TARGET_DIR}/${TARGET}/${PROFILE}/corrosion.dll"
if [ ! -f "${DLL_PATH}" ]; then
    echo "Error: Built library not found at ${DLL_PATH}"
    exit 1
fi

cp "${DLL_PATH}" "${BUNDLE_DIR}/Contents/x86_64-win/${PLUGIN_NAME}.vst3"
echo "Windows VST3 bundle created at: ${BUNDLE_DIR}"

# A CLAP plugin is a single file (the shared library renamed), not a folder.
CLAP_PATH="${PROJECT_DIR}/target/bundled-win/${PLUGIN_NAME}.clap"
rm -rf "${CLAP_PATH}"
cp "${DLL_PATH}" "${CLAP_PATH}"
echo "Windows CLAP plugin created at: ${CLAP_PATH}"

echo "Done."
