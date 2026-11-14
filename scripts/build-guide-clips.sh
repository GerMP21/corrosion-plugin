#!/usr/bin/env bash
# Render the user guide's audio clips and encode them for the site
# (`add-user-guide`). Run after changing the clip plan
# (src/guide/clip_plan.rs) or after a change that moves the sound
# engine-wide; `guide_clip_manifest_is_complete_and_audible` fails while the
# committed clips are stale. Needs opusenc (opus-tools). Not needed by CI or
# the deploy: the clips are committed.
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
out="$repo/web/guide/public/clips"
wavs="$(mktemp -d)"
trap 'rm -rf "$wavs"' EXIT
command -v opusenc >/dev/null || { echo "opusenc not found (apt-get install opus-tools)" >&2; exit 1; }
commit="$(git -C "$repo" rev-parse --short HEAD 2>/dev/null || echo unknown)"
host="$(rustc -vV | sed -n 's/^host: //p')"
cd "$repo"
CARGO_BUILD_TARGET="${CARGO_BUILD_TARGET:-$host}" \
  cargo run --release --no-default-features --bin render_guide_clips -- --out "$wavs" --commit "$commit"
rm -rf "$out"
mkdir -p "$out"
cd "$wavs"
find . -name '*.wav' | sort | while read -r wav; do
  dest="$out/${wav%.wav}.opus"
  mkdir -p "$(dirname "$dest")"
  opusenc --quiet --bitrate 48 "$wav" "$dest"
done
cp clips.json "$out/clips.json"
echo "clips: $(find "$out" -name '*.opus' | wc -l) files, $(du -sh "$out" | cut -f1)"
