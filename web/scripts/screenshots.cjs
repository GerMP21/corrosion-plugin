// Captures the guide's editor screenshots (`write-user-guide-content`
// Decision 4) from the local web app, so the colours are the real ones.
// One image set serves both languages: the images carry no annotations, and
// <Screenshot> draws the numbered callouts over them. Rerun after an editor
// change, then commit the new images.
// Usage: node web/scripts/screenshots.cjs http://localhost:8787/ [out-dir]
// (out-dir defaults to web/guide/src/assets/screenshots; needs ImageMagick's
// `convert` for the WebP files).
const { chromium } = require("playwright");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

/** Canvas regions in CSS pixels of a 1280 px wide editor (the editor scales
 * with its width, so these are fractions of 1280). Measured on the default
 * patch, 2026-11-14. */
const W = 1280;
const REGIONS = {
  editor: null, // the whole canvas
  header: [0, 0, 1280, 145],
  macros: [0, 140, 1280, 135],
  exciter: [0, 272, 438, 1008],
  resonator: [424, 272, 432, 774],
  processing: [844, 272, 436, 928],
  keyboard: [0, 1278, 1280, 130],
};

const wait = (page, ms) => page.waitForTimeout(ms);

(async () => {
  const [base = "http://localhost:8787/", outArg] = process.argv.slice(2);
  const out = path.resolve(outArg ?? path.join(__dirname, "../guide/src/assets/screenshots"));
  fs.mkdirSync(out, { recursive: true });
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "corrosion-shots-"));
  const browser = await chromium.launch({ args: ["--enable-unsafe-swiftshader", "--use-angle=swiftshader"] });
  // A 3200 px window gives a 2560 px editor (80 % of the width), twice the
  // 1280 px the regions are measured at, for sharp text. Not a 1600 px window
  // at deviceScaleFactor 2: the web editor sizes its canvas for a device
  // pixel ratio of 1 and cuts off below 1408 / ratio CSS px (reported to the
  // maintainer, 2026-11-14). The canvas is 1.1 times as tall as it is wide.
  const page = await browser.newPage({ viewport: { width: 3200, height: 3000 }, deviceScaleFactor: 1 });
  const fresh = async () => {
    await page.goto(new URL("play/", base).href);
    await page.waitForFunction(() => window.corrosionEditorReady, null, { timeout: 30000 });
    await wait(page, 1500);
    return page.locator("#editor").boundingBox();
  };
  const written = [];
  const shoot = async (box, name, rect) => {
    const s = box.width / W;
    const clip = rect
      ? { x: box.x + rect[0] * s, y: box.y + rect[1] * s, width: rect[2] * s, height: rect[3] * s }
      : box;
    const png = path.join(tmp, `${name}.png`);
    await page.screenshot({ path: png, clip });
    const webp = path.join(out, `${name}.webp`);
    execFileSync("convert", [png, "-strip", "-quality", "86", webp]);
    written.push(`${name}.webp`);
  };
  const click = (box, x, y) => page.mouse.click(box.x + (x * box.width) / W, box.y + (y * box.width) / W);

  // The default patch, panel by panel.
  let box = await fresh();
  for (const [name, rect] of Object.entries(REGIONS)) await shoot(box, name, rect);

  // The preset browser, open.
  await click(box, 563, 57);
  await wait(page, 800);
  await shoot(box, "preset-browser", [0, 0, 840, 300]);
  await page.keyboard.press("Escape");

  // The Randomizer's mode menu.
  box = await fresh();
  await click(box, 420, 108);
  await wait(page, 800);
  await shoot(box, "randomizer", [340, 0, 460, 260]);
  await page.keyboard.press("Escape");

  // Kit mode with a factory kit: the kit bar and the zone.
  box = await fresh();
  const kit = fs.readFileSync(path.join(__dirname, "../../presets/factory/drums/foundry_kit.corrosion-preset"), "utf8");
  await page.evaluate(async (json) => {
    const editor = await import("./corrosion_web_editor.js");
    editor.open_preset_file(json);
  }, kit);
  await wait(page, 1500);
  box = await page.locator("#editor").boundingBox();
  await shoot(box, "kit", [0, 1300, 1280, box.height * (W / box.width) - 1300]);

  await browser.close();
  fs.rmSync(tmp, { recursive: true, force: true });
  console.log(`${written.length} screenshots in ${path.relative(process.cwd(), out) || "."}: ${written.join(", ")}`);
})();
