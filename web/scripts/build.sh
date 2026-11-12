#!/usr/bin/env bash
# Build the web app into web/dist (`add-web-app`).
set -euo pipefail
web="$(cd "$(dirname "$0")/.." && pwd)"
"$web/scripts/fetch-nih-plug.sh"
cd "$web"
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+simd128" \
  cargo build --release -p corrosion-web-engine --target wasm32-unknown-unknown
rm -rf dist && mkdir -p dist
cp www/* dist/
cp target/wasm32-unknown-unknown/release/corrosion_web_engine.wasm dist/
build="$(git -C "$web" rev-parse --short HEAD 2>/dev/null || echo dev)"
sed -i "s|<span id=\"build\">dev</span>|<span id=\"build\">$build</span>|" dist/index.html
ls -l dist
