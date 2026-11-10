#!/usr/bin/env bash
set -euo pipefail

PLUGIN_NAME="Corrosion"
TARGET="x86_64-unknown-linux-gnu"
PROFILE="${1:-release}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
TARGET_DIR="${PROJECT_DIR}/../corrosion-target"

echo "Building ${PLUGIN_NAME} for ${TARGET} (${PROFILE})..."
cargo build --target "${TARGET}" --"${PROFILE}" --lib

BUNDLE_DIR="${PROJECT_DIR}/target/bundled/${PLUGIN_NAME}.vst3"
rm -rf "${BUNDLE_DIR}"
mkdir -p "${BUNDLE_DIR}/Contents/x86_64-linux"

SO_PATH="${TARGET_DIR}/${TARGET}/${PROFILE}/libcorrosion.so"
if [ ! -f "${SO_PATH}" ]; then
    echo "Error: Built library not found at ${SO_PATH}"
    exit 1
fi

cp "${SO_PATH}" "${BUNDLE_DIR}/Contents/x86_64-linux/${PLUGIN_NAME}.so"
echo "VST3 bundle created at: ${BUNDLE_DIR}"

# A CLAP plugin is a single file (the shared library renamed), not a folder.
CLAP_PATH="${PROJECT_DIR}/target/bundled/${PLUGIN_NAME}.clap"
rm -rf "${CLAP_PATH}"
cp "${SO_PATH}" "${CLAP_PATH}"
echo "CLAP plugin created at: ${CLAP_PATH}"

echo "Done."
