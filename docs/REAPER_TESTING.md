# Testing the plugin in REAPER from a Claude Code session

Use this guide to check the editor and the sound in a real host from a cloud Claude Code session, which is a Linux container with no screen. REAPER for Linux runs on a virtual X display. Its startup script loads the CLAP plugin and opens its window. Then `xdotool` clicks and types, and ImageMagick takes screenshots.

These checks are manual. Nothing here runs in CI (see "DAW smoke automation" in `docs/BACKLOG.md`).

Everything lives under `/tmp/claude-0/`, which is lost when the container is recycled. Each new session redoes steps 1–3; they take about five minutes.

## 1. System packages

```bash
apt-get install -y xserver-xorg-core xserver-xorg-video-dummy libgl1-mesa-dri libglx-mesa0 mesa-utils xdotool imagemagick
```

**Use Xorg's dummy driver, not Xvfb.** egui needs an OpenGL context. Earlier sessions tried Xvfb first and only got the editor drawing on the dummy driver.

## 2. Virtual display on `:98`

```bash
mkdir -p /tmp/claude-0
cat > /tmp/claude-0/xorg-dummy.conf <<'EOF'
Section "Device"
    Identifier "dummy"
    Driver "dummy"
    VideoRam 256000
EndSection
Section "Monitor"
    Identifier "monitor"
    HorizSync 5.0-1000.0
    VertRefresh 5.0-200.0
    Modeline "1920x1200" 154.00 1920 1968 2000 2080 1200 1203 1209 1235 +hsync -vsync
EndSection
Section "Screen"
    Identifier "screen"
    Device "dummy"
    Monitor "monitor"
    DefaultDepth 24
    SubSection "Display"
        Depth 24
        Modes "1920x1200"
    EndSubSection
EndSection
EOF
```

```bash
pgrep -x Xorg >/dev/null || (Xorg :98 -noreset -config /tmp/claude-0/xorg-dummy.conf -logfile /tmp/claude-0/xorg98.log >/dev/null 2>&1 &)
```

```bash
DISPLAY=:98 xdotool getdisplaygeometry
```

The last command should print the screen size. Xorg can take a few seconds to start.

## 3. REAPER

Download the Linux build into its own directory. It runs from there unpacked; it doesn't need installing.

```bash
mkdir -p /tmp/claude-0/reaper-dl && cd /tmp/claude-0/reaper-dl && curl -fsSLO https://www.reaper.fm/files/7.x/reaper785_linux_x86_64.tar.xz && tar -xf reaper785_linux_x86_64.tar.xz
```

`reaper785` is the 7.85 build that was used. For a newer one, take the current file name from <https://www.reaper.fm/download.php>. REAPER runs in its evaluation mode, which shows an "About REAPER" window at start.

Add a startup script. It inserts a track, adds Corrosion to it, opens the plugin window, and logs what it found:

```bash
cat > /tmp/claude-0/reaper-dl/reaper_linux_x86_64/REAPER/Scripts/__startup.lua <<'EOF'
local log = io.open("/tmp/claude-0/reaper-startup.log", "w")
reaper.InsertTrackAtIndex(0, true)
local track = reaper.GetTrack(0, 0)
local idx = reaper.TrackFX_AddByName(track, "CLAP: Corrosion", false, -1)
log:write("fx index ", tostring(idx), "\n")
if idx >= 0 then
  local _, name = reaper.TrackFX_GetFXName(track, idx, "")
  log:write("fx name ", name, "\n")
  reaper.TrackFX_Show(track, idx, 3)
end
log:close()
EOF
```

`mkdir -p` the `Scripts` folder first if it doesn't exist.

## 4. Build the plugin for the virtual display

The dummy X server has no sRGB framebuffer config, so the release plugin's editor fails to open there with `InvalidFBConfig`. To work around it, build from a separate worktree against a copy of NIH-plug whose egui editor asks for `srgb: false`. This build is for screenshots only and is never shipped. Colours look slightly different from a real screen.

Make the patched NIH-plug copy once per container, at the revision `Cargo.toml` pins:

```bash
git clone -q https://github.com/robbert-vdh/nih-plug /tmp/claude-0/nihplug && git -C /tmp/claude-0/nihplug checkout -q 28b149ec4d62757d0b448809148a0c3ca6e09a95 && sed -i 's/srgb: true,/srgb: false,/' /tmp/claude-0/nihplug/nih_plug_egui/src/editor.rs
```

Make a worktree of the commit to test, and point it at the copy:

```bash
git worktree add --detach /tmp/claude-0/shotwt HEAD && cat >> /tmp/claude-0/shotwt/Cargo.toml <<'EOF'

[patch."https://github.com/robbert-vdh/nih-plug"]
nih_plug = { path = "/tmp/claude-0/nihplug" }
nih_plug_egui = { path = "/tmp/claude-0/nihplug/nih_plug_egui" }
EOF
```

Build and install it as the CLAP plugin. On Linux, a CLAP plugin is the shared library renamed:

```bash
cd /tmp/claude-0/shotwt && CARGO_BUILD_TARGET=x86_64-unknown-linux-gnu CARGO_TARGET_DIR=/tmp/claude-0/shot-target cargo build --release --lib
```

```bash
mkdir -p ~/.clap && cp /tmp/claude-0/shot-target/x86_64-unknown-linux-gnu/release/libcorrosion.so ~/.clap/Corrosion.clap
```

To test new changes, refresh the worktree (`git -C /tmp/claude-0/shotwt checkout --detach <commit>`, keeping the `[patch]` lines), then rebuild and copy again. When you're done, remove it with `git worktree remove --force /tmp/claude-0/shotwt`.

## 5. Run REAPER and look at the editor

```bash
pkill -f REAPER/reaper; rm -f /tmp/claude-0/reaper-startup.log; (DISPLAY=:98 nohup /tmp/claude-0/reaper-dl/reaper_linux_x86_64/REAPER/reaper -nosplash -new > /tmp/claude-0/reaper.log 2>&1 &)
```

After 10–20 seconds, `/tmp/claude-0/reaper-startup.log` should say `fx name CLAPi: Corrosion (Synaptech Solutions)`. If `fx index` is `-1`, REAPER didn't find the CLAP. Check `~/.clap/Corrosion.clap`, and delete REAPER's CLAP cache (the `reaper-clap-*.ini` file in `~/.config/REAPER/`) so it rescans.

Close the About window, put the plugin window at the top left, and take a screenshot:

```bash
export DISPLAY=:98
xdotool search --onlyvisible --name 'About REAPER' windowclose
W=$(xdotool search --onlyvisible --name 'CLAPi: Corrosion' | head -1); xdotool windowactivate $W; xdotool windowmove $W 0 0
import -window root -crop 810x658+0+0 +repage /tmp/claude-0/reaper-75.png
```

At the default 75 % UI Scale, the window is 810 × 658 (the editor's 810 × 630 plus REAPER's title bar), so the crop above holds it exactly. Open the PNG with the Read tool to look at it. For a screenshot the maintainer will see, copy it to the session's scratchpad and send it.

Driving the editor:
- **Click:** `xdotool mousemove X Y click 1`. Coordinates are screen pixels, and the window sits at 0,0.
- **Scroll the panels:** `xdotool mousemove 400 400 click 5`, repeated (button 4 scrolls up).
- **Drag:** `xdotool mousemove X Y mousedown 1 mousemove X2 Y2 mouseup 1`.
- **Type** (the preset name, for example): `xdotool type --delay 80 'My Preset'`.
- **Play notes:** with the pointer over the editor, use the computer keys `A W S E D F T G Y H U J K` (C4–C5) through `xdotool keydown a` / `keyup a`. You can also click the piano strip at the bottom.
- **Hover a help badge** (ⓘ): `mousemove` onto it, wait about a second, then take the screenshot.

Between steps, wait with `timeout 1 tail -f /dev/null`: this session's shell blocks a plain `sleep`. egui redraws a frame or two after an input.

## 6. Other REAPER actions through Lua

Other one-off actions go through Lua scripts, run in the same REAPER:

```bash
DISPLAY=:98 /tmp/claude-0/reaper-dl/reaper_linux_x86_64/REAPER/reaper -nonewinst /tmp/claude-0/some-action.lua
```

Examples used in past sessions:
- open the plugin window: `reaper.TrackFX_Show(reaper.GetTrack(0, 0), 0, 3)`;
- close it: `reaper.TrackFX_Show(reaper.GetTrack(0, 0), 0, 2)`;
- save the project: `reaper.Main_SaveProjectEx(0, "/tmp/claude-0/proj.rpp", 0)`;
- reopen it to check that state restores: `reaper.Main_openProject("/tmp/claude-0/proj.rpp")`;
- read a parameter: `reaper.TrackFX_GetParamNormalized(track, 0, index)`, written to a log file because a script has no console here.

## Limits

- **No audio output.** REAPER runs with no audio device, so you can't hear anything here. Check the sound with the offline renderer and the tests, or with the web app's meter (`web/scripts/smoke.cjs`).
- **Colours and font rendering** differ slightly from a real screen (`srgb: false`, Mesa's software renderer).
- **Linux only.** It says nothing about the Windows or macOS builds or their hosts.
- In the change's notes, say what was checked this way ("checked in REAPER (Linux, 75 %): …"), as earlier changes did.
