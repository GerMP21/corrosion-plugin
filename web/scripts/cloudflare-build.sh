#!/usr/bin/env bash
# Build command for Cloudflare Workers Builds (`add-web-app`). Cloudflare's
# build image has no Rust, so this installs what the web build needs when it
# is missing (a minimal rustup toolchain with the wasm32 target, and the
# prebuilt wasm-bindgen CLI at the version web/Cargo.lock pins), then runs
# scripts/build.sh. Workers Builds settings: root directory `web`, build
# command `bash scripts/cloudflare-build.sh`, deploy command
# `npx wrangler deploy`.
set -euo pipefail
web="$(cd "$(dirname "$0")/.." && pwd)"
tools="$web/.tools"
mkdir -p "$tools/bin"
export PATH="$tools/bin:$HOME/.cargo/bin:$PATH"

if ! command -v cargo >/dev/null; then
  echo "Installing a minimal Rust toolchain"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
  export PATH="$HOME/.cargo/bin:$PATH"
fi
rustup target add wasm32-unknown-unknown

bindgen_version="$(grep -A1 '^name = "wasm-bindgen"$' "$web/Cargo.lock" | sed -n 's/^version = "\(.*\)"$/\1/p')"
if [ "$(wasm-bindgen --version 2>/dev/null | awk '{print $2}')" != "$bindgen_version" ]; then
  echo "Installing wasm-bindgen $bindgen_version"
  name="wasm-bindgen-$bindgen_version-x86_64-unknown-linux-musl"
  curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$bindgen_version/$name.tar.gz" \
    | tar -xz -C "$tools"
  cp "$tools/$name/wasm-bindgen" "$tools/bin/"
fi

"$web/scripts/build.sh"
