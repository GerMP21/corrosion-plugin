#!/usr/bin/env bash
# Build the web app into web/dist (`add-web-app`): the engine wasm (loaded
# by the AudioWorklet, no JS glue), the editor wasm with its wasm-bindgen
# glue, and the static page. Needs the wasm32-unknown-unknown target and
# wasm-bindgen-cli at the version in web/Cargo.lock.
set -euo pipefail
web="$(cd "$(dirname "$0")/.." && pwd)"
"$web/scripts/fetch-nih-plug.sh"
cd "$web"
target=wasm32-unknown-unknown
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+simd128" \
  cargo build --release --target "$target" -p corrosion-web-engine -p corrosion-web-editor
rm -rf dist && mkdir -p dist
cp www/* dist/
cp "target/$target/release/corrosion_web_engine.wasm" dist/
wasm-bindgen --target web --no-typescript --out-dir dist \
  "target/$target/release/corrosion_web_editor.wasm"
build="$(git -C "$web" rev-parse --short HEAD 2>/dev/null || echo dev)"
sed -i "s|<span id=\"build\">dev</span>|<span id=\"build\">$build</span>|" dist/index.html
ls -l dist
