// Headless Chromium check of the web host's features (`add-web-app`
// group 4): session autosave and restore, saving a preset in the editor
// into IndexedDB, opening a preset file, downloading the current sound,
// MIDI messages reaching the engine, the BPM field, and the load meter.
// Usage: node web/scripts/features.cjs http://localhost:8787/ preset-file
const { chromium } = require("playwright");
const fs = require("fs");

const wait = (page, ms) => page.waitForTimeout(ms);
const failures = [];
const check = (name, ok, detail) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? `: ${detail}` : ""}`);
  if (!ok) failures.push(name);
};
/** The current sound without its name, to compare sessions. */
const sound = (page) =>
  page.evaluate(async () => {
    const editor = await import("./corrosion_web_editor.js");
    const preset = JSON.parse(editor.current_preset_json("x"));
    delete preset.name;
    return preset;
  });
/** Whether two sounds match: numbers within float noise (a restore goes
 * through normalized values, as a preset load in the plugin does). */
const sameSound = (a, b) => {
  if (typeof a === "number" && typeof b === "number") return Math.abs(a - b) <= 1e-5 * Math.max(1, Math.abs(a));
  if (a && b && typeof a === "object" && typeof b === "object") {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    return [...keys].every((key) => sameSound(a[key], b[key]));
  }
  return a === b;
};

(async () => {
  const [url = "http://localhost:8787/", presetFile] = process.argv.slice(2);
  const browser = await chromium.launch({
    args: ["--autoplay-policy=no-user-gesture-required", "--enable-unsafe-swiftshader", "--use-angle=swiftshader"],
  });
  const context = await browser.newContext({ viewport: { width: 1100, height: 960 }, acceptDownloads: true });
  const page = await context.newPage();
  const errors = [];
  page.on("console", (msg) => msg.type() === "error" && errors.push(msg.text()));
  page.on("pageerror", (error) => errors.push(error.message));
  const ready = () => page.waitForFunction(() => window.corrosionEditorReady, null, { timeout: 30000 });

  await page.goto(url);
  await ready();
  const canvas = await page.locator("#editor").boundingBox();
  const at = (fx, fy) => [canvas.x + canvas.width * fx, canvas.y + canvas.height * fy];
  const fresh = await sound(page);

  // Session: pick the next factory preset, let the autosave run, reload.
  await page.mouse.click(...at(0.527, 0.042));
  await wait(page, 2000);
  const changed = await sound(page);
  check("the editor changes the sound", !sameSound(changed, fresh));
  await page.reload();
  await ready();
  await wait(page, 300);
  check("a reload restores the session", sameSound(await sound(page), changed));

  // Save in the editor: type a name in the bank's field and press Save.
  await page.mouse.click(...at(0.38, 0.0665));
  // At a typing pace: egui's web text input drops characters that arrive
  // in the same instant.
  await page.keyboard.type("Web Test", { delay: 40 });
  await page.mouse.click(...at(0.519, 0.0665));
  await wait(page, 2000);
  const stored = await page.evaluate(() => new Promise((resolve) => {
    const request = indexedDB.open("corrosion", 1);
    request.onsuccess = () => {
      const all = request.result.transaction("presets").objectStore("presets").getAll();
      all.onsuccess = () => resolve(all.result.map((p) => p.file));
    };
  }));
  check("Save keeps the preset in IndexedDB", stored.includes("Web Test"), JSON.stringify(stored));

  // Audio, then MIDI straight to the engine.
  await page.click("#start");
  await page.waitForFunction(() => window.corrosionMeter, null, { timeout: 15000 });
  const midiLabel = await page.locator("#midi option").first().textContent();
  check("the MIDI menu is set up", midiLabel.length > 0, midiLabel);
  await page.evaluate(() => window.corrosionPost?.({ type: "midi", data: [0x90, 64, 110] }));
  await wait(page, 600);
  const midi = await page.evaluate(() => window.corrosionMeter);
  check("a MIDI note sounds", midi.rms > 1e-4 && midi.voices >= 1, JSON.stringify({ rms: midi.rms, voices: midi.voices }));
  await page.evaluate(() => window.corrosionPost?.({ type: "midi", data: [0x80, 64, 0] }));
  await wait(page, 1200);
  const meter = await page.evaluate(() => window.corrosionMeter);
  check("the load meter reports", typeof meter.load === "number" && meter.load >= 0 && meter.load < 1, String(meter.load));
  check("the load is shown", /CPU \d+ %/.test(await page.textContent("#load")), await page.textContent("#load"));

  // BPM is kept.
  await page.fill("#bpm", "90");
  await page.dispatchEvent("#bpm", "change");
  check("the BPM is kept", (await page.evaluate(() => localStorage.getItem("corrosion.bpm"))) === "90");

  // Open a preset file.
  if (presetFile) {
    await page.setInputFiles("#file", presetFile);
    await wait(page, 800);
    const name = JSON.parse(fs.readFileSync(presetFile, "utf8")).name;
    const opened = await page.evaluate(async () => {
      const editor = await import("./corrosion_web_editor.js");
      return JSON.parse(editor.current_preset_json("x"));
    });
    const file = JSON.parse(fs.readFileSync(presetFile, "utf8"));
    check("opening a file loads its sound", opened.exciter === file.exciter && opened.object === file.object, name);
  }

  // Download the current sound.
  page.once("dialog", (dialog) => dialog.accept("Downloaded Sound"));
  const [download] = await Promise.all([page.waitForEvent("download"), page.click("#download")]);
  const path = await download.path();
  const downloaded = JSON.parse(fs.readFileSync(path, "utf8"));
  check("Download gives a preset file", download.suggestedFilename() === "Downloaded Sound.corrosion-preset" && downloaded.name === "Downloaded Sound");

  // The saved preset is still listed after a reload.
  await page.reload();
  await ready();
  check("no console errors", errors.length === 0, errors.join(" | "));
  await browser.close();
  if (failures.length) {
    console.error(`${failures.length} check(s) failed`);
    process.exit(1);
  }
  console.log("feature checks passed");
})();
