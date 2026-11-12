// Headless Chromium check of the web app (`add-web-app`): the editor
// starts in its canvas, audio starts, a key on the editor's strip and a
// computer key sound, a preset chosen in the editor reaches the engine, and
// no console error is logged.
// Usage: node web/scripts/smoke.cjs http://localhost:8787/ [screenshot.png]
const { chromium } = require("playwright");

const meter = (page) => page.evaluate(() => window.corrosionMeter);
const wait = (page, ms) => page.waitForTimeout(ms);

(async () => {
  const url = process.argv[2] ?? "http://localhost:8787/";
  const browser = await chromium.launch({
    args: ["--autoplay-policy=no-user-gesture-required", "--enable-unsafe-swiftshader", "--use-angle=swiftshader"],
  });
  const page = await browser.newPage({ viewport: { width: 1100, height: 960 } });
  const errors = [];
  page.on("console", (msg) => msg.type() === "error" && errors.push(msg.text()));
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url);
  await page.waitForFunction(() => window.corrosionEditorReady, null, { timeout: 30000 });
  await page.click("#start");
  await page.waitForFunction(() => window.corrosionMeter, null, { timeout: 15000 });
  await wait(page, 400);
  const silent = await meter(page);

  // The canvas fills 1080 px of the 1100 px viewport; the strip is its
  // bottom row, and C4 (the "A" key) sits just right of its middle.
  const canvas = await page.locator("#editor").boundingBox();
  const at = (fx, fy) => [canvas.x + canvas.width * fx, canvas.y + canvas.height * fy];
  await page.mouse.move(...at(0.516, 0.985));
  await page.mouse.down();
  await wait(page, 600);
  const strip = await meter(page);
  if (process.argv[3]) await page.screenshot({ path: process.argv[3] });
  await page.mouse.up();

  await page.keyboard.down("a");
  await wait(page, 500);
  const key = await meter(page);
  await page.keyboard.up("a");

  // Next preset (the bank's "›"), then the same key: a different sound.
  await wait(page, 1500);
  await page.mouse.click(...at(0.527, 0.042));
  await wait(page, 300);
  await page.keyboard.down("a");
  await wait(page, 500);
  const nextPreset = await meter(page);
  await page.keyboard.up("a");
  await browser.close();

  const result = { silent, strip, key, nextPreset, errors };
  console.log(JSON.stringify(result));
  const ok =
    silent.rms === 0 &&
    strip.rms > 1e-4 &&
    key.rms > 1e-4 &&
    nextPreset.rms > 1e-4 &&
    Math.abs(nextPreset.rms - key.rms) > 1e-3 &&
    errors.length === 0;
  if (!ok) {
    console.error("smoke check failed");
    process.exit(1);
  }
  console.log("smoke check passed");
})();
