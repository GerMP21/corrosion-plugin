#!/usr/bin/env bash
set -euo pipefail

# Local release-hardening verification script.
# This mirrors the repository CI lane closely enough that developers can
# reproduce failures before pushing changes.

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RENDER_DIR="/tmp/corrosion-local-verify-render"

cd "${PROJECT_DIR}"

echo "==> rustfmt"
cargo fmt --check

echo "==> clippy (workspace, no default features)"
cargo clippy --workspace --all-targets --no-default-features -- -D warnings

echo "==> tests (workspace, no default features)"
cargo test --workspace --no-default-features

echo "==> tests (native Linux library lane)"
cargo test --lib --target x86_64-unknown-linux-gnu

echo "==> offline renderer smoke"
rm -rf "${RENDER_DIR}"
cargo run --target x86_64-unknown-linux-gnu --bin render -- --suite family --out-dir "${RENDER_DIR}"

echo "==> rendered wav validation"
python3 scripts/check_wav.py "${RENDER_DIR}/pipe_comparison.wav"

echo "==> linux bundle"
scripts/bundle.sh linux release
scripts/check_bundle.sh linux

echo "Local verification passed."
