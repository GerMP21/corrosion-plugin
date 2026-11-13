// Captures the editor for the home page (`add-site-shell`) from the local
// web app, so the colours are the real ones (unlike the REAPER screenshot
// build). Rerun after an editor change, then commit the new image.
// Usage: node web/scripts/home-screenshot.cjs http://localhost:8787/ /tmp/editor.png
// then: convert /tmp/editor.png -resize 1600x -strip -quality 86 web/site/assets/editor.webp
const { chromium } = require("playwright");

(async () => {
  const [base = "http://localhost:8787/", out = "editor.png"] = process.argv.slice(2);
  const browser = await chromium.launch({ args: ["--enable-unsafe-swiftshader", "--use-angle=swiftshader"] });
  // The editor takes 80 % of the width: a 1600 px window gives a 1280 px
  // editor, captured at twice the pixels for sharp text.
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 2 });
  await page.goto(new URL("play/", base).href);
  await page.waitForFunction(() => window.corrosionEditorReady, null, { timeout: 30000 });
  await page.waitForTimeout(1500);
  const canvas = page.locator("#editor");
  const box = await canvas.boundingBox();
  // The whole editor, piano strip included.
  await page.screenshot({ path: out, clip: { x: box.x, y: box.y, width: box.width, height: box.height }, fullPage: true });
  console.log(`${out}: ${Math.round(box.width)} × ${Math.round(box.height)} CSS px`);
  await browser.close();
})();
