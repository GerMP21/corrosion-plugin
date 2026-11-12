#!/usr/bin/env bash
# Put NIH-plug, at the revision the plugin pins in Cargo.lock, in
# web/vendor/nih-plug and apply web/nih-plug-public-setters.patch.
set -euo pipefail
web="$(cd "$(dirname "$0")/.." && pwd)"
rev="$(grep -A2 '^name = "nih_plug"$' "$web/../Cargo.lock" | sed -n 's/.*#\([0-9a-f]\{40\}\)".*/\1/p' | head -1)"
dest="$web/vendor/nih-plug"
if [ -f "$dest/.patched-$rev" ]; then exit 0; fi
rm -rf "$dest"
git clone --quiet https://github.com/robbert-vdh/nih-plug "$dest"
git -C "$dest" checkout --quiet "$rev"
git -C "$dest" apply "$web/nih-plug-public-setters.patch"
touch "$dest/.patched-$rev"
echo "nih-plug $rev patched in $dest"
