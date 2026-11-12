// Headless Chromium check of the web app (`add-web-app`): the page loads,
// audio starts, a note sounds (RMS > 0), and no console error is logged.
// Usage: node web/scripts/smoke.cjs http://localhost:8787/
const { chromium } = require("playwright");

(async () => {
  const url = process.argv[2] ?? "http://localhost:8787/";
  const browser = await chromium.launch({ args: ["--autoplay-policy=no-user-gesture-required"] });
  const page = await browser.newPage();
  const errors = [];
  page.on("console", (msg) => msg.type() === "error" && errors.push(msg.text()));
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(url);
  await page.click("#start");
  await page.waitForFunction(() => !document.getElementById("note").disabled, null, { timeout: 15000 });
  const silent = await page.evaluate(() => new Promise((r) => setTimeout(() => r(window.corrosionMeter), 600)));
  await page.dispatchEvent("#note", "pointerdown");
  const played = await page.evaluate(() => new Promise((r) => setTimeout(() => r(window.corrosionMeter), 800)));
  await browser.close();
  console.log(JSON.stringify({ silent, played, errors }));
  const ok = silent && silent.rms === 0 && played && played.rms > 1e-4 && errors.length === 0;
  if (!ok) {
    console.error("smoke check failed");
    process.exit(1);
  }
  console.log("smoke check passed");
})();
