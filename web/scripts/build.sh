#!/usr/bin/env bash
# Build the site into web/dist: the home page (`add-site-shell`), the web
# app under play/ (`add-web-app`): the engine wasm (loaded by the
# AudioWorklet, no JS glue), the editor wasm with its wasm-bindgen glue, and
# the static page, and the user guide under wiki/ (`add-user-guide`). Needs
# the wasm32-unknown-unknown target, wasm-bindgen-cli at the version in
# web/Cargo.lock, and Node at the version web/guide/package.json requires.
set -euo pipefail
web="$(cd "$(dirname "$0")/.." && pwd)"
"$web/scripts/fetch-nih-plug.sh"
cd "$web"
target=wasm32-unknown-unknown
out="${CARGO_TARGET_DIR:-$web/target}"
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+simd128" \
  cargo build --release --target "$target" -p corrosion-web-engine -p corrosion-web-editor
# Site layout (`add-site-shell`): the home page and site-wide files at the
# root, the shared design tokens under theme/, the app under play/.
rm -rf dist && mkdir -p dist/play dist/theme
cp -r site/. dist/
cp theme/* dist/theme/
cp www/* dist/play/
cp "$out/$target/release/corrosion_web_engine.wasm" dist/play/
wasm-bindgen --target web --no-typescript --out-dir dist/play \
  "$out/$target/release/corrosion_web_editor.wasm"
build="$(git -C "$web" rev-parse --short HEAD 2>/dev/null || echo dev)"
for page in dist/index.html dist/404.html dist/play/index.html; do
  sed -i "s|<span id=\"build\">dev</span>|<span id=\"build\">$build</span>|" "$page"
done
# The guide (`add-user-guide`): Astro writes it into dist/wiki/ itself.
(cd guide && npm ci --no-audit --no-fund && npm run check && npm run build)
du -sh dist/* | sort -h
