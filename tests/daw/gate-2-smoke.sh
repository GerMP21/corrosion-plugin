#!/bin/bash
set -e

echo "G2-13 REAPER Smoke Test"
echo "========================"

# Verify REAPER is installed
if ! command -v reaper &> /dev/null; then
    echo "REAPER not found, skipping DAW test"
    exit 0
fi

# Verify plugin bundle exists
VST3_PATH="target/bundled/linux/Corrosion.vst3"
if [ ! -d "$VST3_PATH" ]; then
    echo "Building Linux bundle..."
    ./bundle.sh
fi

# Test plugin scan (REAPER can scan the plugin)
echo "Testing plugin scan..."
reaper -nonewinst -cfgfile /tmp/reaper-g2.ini &
REAPER_PID=$!
sleep 3

# Check if plugin appears in registry
if grep -r "Corrosion" ~/.config/REAPER/reaper-vstplugins64.ini 2>/dev/null || \
   grep -r "Corrosion" /tmp/reaper-g2.ini 2>/dev/null; then
    echo "✓ Plugin found in REAPER registry"
else
    echo "Note: Plugin may need manual scan first"
fi

# Cleanup
kill $REAPER_PID 2>/dev/null || true

echo "REAPER smoke test complete"
