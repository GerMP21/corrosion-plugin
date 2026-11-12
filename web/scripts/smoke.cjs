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

  // The page scrolls, not the editor: the canvas is as tall as the whole
  // editor, the page is taller than the window, and the wheel over the
  // canvas scrolls the page.
  const layout = await page.evaluate(() => ({
    canvas: document.getElementById("editor").getBoundingClientRect().height,
    page: document.documentElement.scrollHeight,
    window: window.innerHeight,
  }));
  const top = await page.locator("#editor").boundingBox();
  await page.mouse.move(top.x + top.width / 2, top.y + top.height / 2);
  await page.mouse.wheel(0, 400);
  await wait(page, 300);
  const scrolled = await page.evaluate(() => window.scrollY);
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  await wait(page, 300);

  // The editor fills the canvas's width and scales with it, so positions
  // are fractions of the width, from the canvas's top or bottom edge. The
  // strip is the bottom row; C4 (the "A" key) sits at its middle.
  let canvas = await page.locator("#editor").boundingBox();
  const bottomY = await page.evaluate(() => window.scrollY);
  const fromBottom = (fx, fy) => [canvas.x + canvas.width * fx, canvas.y + canvas.height - canvas.width * fy];
  const fromTop = (fx, fy) => [canvas.x + canvas.width * fx, canvas.y + canvas.width * fy];
  await page.mouse.move(...fromBottom(0.5, 0.03));
  await page.mouse.down();
  await wait(page, 600);
  const strip = await meter(page);
  // eframe focuses the canvas on a click; that must not scroll the page.
  const clickScroll = (await page.evaluate(() => window.scrollY)) - bottomY;
  if (process.argv[3]) await page.screenshot({ path: process.argv[3] });
  await page.mouse.up();

  await page.keyboard.down("a");
  await wait(page, 500);
  const key = await meter(page);
  await page.keyboard.up("a");

  // Next preset (the bank's "›"), then the same key: a different sound.
  await wait(page, 1500);
  await page.evaluate(() => window.scrollTo(0, 0));
  await wait(page, 300);
  canvas = await page.locator("#editor").boundingBox();
  await page.mouse.click(...fromTop(0.565, 0.0355));
  await wait(page, 300);
  await page.keyboard.down("a");
  await wait(page, 500);
  const nextPreset = await meter(page);
  await page.keyboard.up("a");
  await browser.close();

  const result = { layout, scrolled, clickScroll, silent, strip, key, nextPreset, errors };
  console.log(JSON.stringify(result));
  const ok =
    layout.canvas > layout.window * 0.9 &&
    layout.page > layout.window &&
    scrolled > 0 &&
    clickScroll === 0 &&
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
