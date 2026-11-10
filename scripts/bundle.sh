#!/usr/bin/env bash
# Build the VST3 and CLAP bundles for one platform
# (`extend-ci-windows-linux-macos`).
#
#   scripts/bundle.sh <linux|windows|macos> [release|debug]
#
# Output: target/bundled/<platform>/Corrosion.vst3 and Corrosion.clap, in the
# layout each format needs on that platform. Check it with
# scripts/check_bundle.sh <platform>.
#
# - linux:   x86_64-unknown-linux-gnu.
# - windows: x86_64-pc-windows-msvc (the CI's shipped build); set
#            CORROSION_WINDOWS_TARGET=x86_64-pc-windows-gnu for the mingw
#            build of the maintainer's machine (bundle-win.sh does).
# - macos:   aarch64 and x86_64, joined with lipo into a universal binary,
#            then ad-hoc signed (Apple Silicon refuses unsigned code). Not
#            notarized.
#
# The library is looked up under $CARGO_TARGET_DIR, or ../corrosion-target
# (the default of .cargo/config.toml).
set -euo pipefail

PLATFORM="${1:?usage: bundle.sh <linux|windows|macos> [release|debug]}"
PROFILE="${2:-release}"
NAME="Corrosion"
LIB="corrosion"

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"
TARGET_DIR="${CARGO_TARGET_DIR:-${PROJECT_DIR}/../corrosion-target}"
OUT="${PROJECT_DIR}/target/bundled/${PLATFORM}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

case "$PROFILE" in
    release) PROFILE_FLAG=(--release) ;;
    debug) PROFILE_FLAG=() ;;
    *) echo "bundle.sh: profile must be release or debug" >&2; exit 1 ;;
esac

# Build the library for one target and print its path.
build() {
    local target="$1" file="$2"
    echo "==> cargo build --lib --target ${target} (${PROFILE})" >&2
    cargo build --lib --target "$target" "${PROFILE_FLAG[@]}" >&2
    local path="${TARGET_DIR}/${target}/${PROFILE}/${file}"
    [ -f "$path" ] || { echo "bundle.sh: built library not found at ${path}" >&2; exit 1; }
    echo "$path"
}

# macOS bundle metadata: Info.plist and PkgInfo for a bundle of `kind`
# (vst3 or clap).
mac_metadata() {
    local contents="$1" kind="$2"
    cat > "${contents}/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>${NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.corrosion.corrosion.${kind}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${NAME}</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST
    printf 'BNDL????' > "${contents}/PkgInfo"
}

rm -rf "$OUT"
mkdir -p "$OUT"

case "$PLATFORM" in
    linux)
        lib="$(build x86_64-unknown-linux-gnu "lib${LIB}.so")"
        mkdir -p "${OUT}/${NAME}.vst3/Contents/x86_64-linux"
        cp "$lib" "${OUT}/${NAME}.vst3/Contents/x86_64-linux/${NAME}.so"
        cp "$lib" "${OUT}/${NAME}.clap"
        ;;
    windows)
        lib="$(build "${CORROSION_WINDOWS_TARGET:-x86_64-pc-windows-msvc}" "${LIB}.dll")"
        mkdir -p "${OUT}/${NAME}.vst3/Contents/x86_64-win"
        cp "$lib" "${OUT}/${NAME}.vst3/Contents/x86_64-win/${NAME}.vst3"
        cp "$lib" "${OUT}/${NAME}.clap"
        ;;
    macos)
        arm="$(build aarch64-apple-darwin "lib${LIB}.dylib")"
        intel="$(build x86_64-apple-darwin "lib${LIB}.dylib")"
        universal="${OUT}/lib${LIB}-universal.dylib"
        lipo -create "$arm" "$intel" -output "$universal"
        for kind in vst3 clap; do
            contents="${OUT}/${NAME}.${kind}/Contents"
            mkdir -p "${contents}/MacOS"
            cp "$universal" "${contents}/MacOS/${NAME}"
            mac_metadata "$contents" "$kind"
            codesign --force --deep --sign - "${OUT}/${NAME}.${kind}"
        done
        rm "$universal"
        ;;
    *)
        echo "bundle.sh: unknown platform '${PLATFORM}' (linux, windows or macos)" >&2
        exit 1
        ;;
esac

echo "Bundles in ${OUT}:"
ls -1 "$OUT"
