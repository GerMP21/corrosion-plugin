// Headless check of the site shell (`add-site-shell`): the home page's three
// calls to action are visible on a 1280 × 720 window and resolve, /play
// redirects to /play/, an unknown path is a themed 404 linking home, and the
// home page doesn't scroll sideways at phone width.
// Usage: node web/scripts/site.cjs http://localhost:8787/
// The releases link points at GitHub; only its presence is checked unless
// CHECK_EXTERNAL=1.
const { chromium } = require("playwright");

const failures = [];
const check = (ok, what) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
  if (!ok) failures.push(what);
};

(async () => {
  const base = process.argv[2] ?? "http://localhost:8787/";
  const at = (path) => new URL(path, base).href;
  const browser = await chromium.launch();

  // Home page and its calls to action, above the fold.
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (msg) => msg.type() === "error" && errors.push(msg.text()));
  const home = await page.goto(at("/"));
  check(home && home.status() === 200, "home page returns 200");
  const ctas = [
    ["play", 'a[data-cta="play"]', "/play/"],
    ["download", 'a[data-cta="download"]', "github.com/"],
    ["guide", 'a[data-cta="guide"]', "/wiki/"],
  ];
  for (const [name, selector, target] of ctas) {
    const link = page.locator(selector);
    const count = await link.count();
    check(count === 1, `one "${name}" call to action`);
    if (count !== 1) continue;
    const box = await link.boundingBox();
    check(box && box.y >= 0 && box.y + box.height <= 720, `"${name}" visible without scrolling`);
    const href = await link.getAttribute("href");
    check(href && href.includes(target), `"${name}" links to ${target} (${href})`);
    const external = href && /^https?:/.test(href) && !href.startsWith(base);
    if (!external || process.env.CHECK_EXTERNAL === "1") {
      const response = await page.request.get(new URL(href, page.url()).href, { maxRedirects: 0 });
      check(response.status() === 200, `"${name}" resolves (${response.status()})`);
    }
  }
  const footer = await page.locator("footer").innerText().catch(() => "");
  check(/GPL/i.test(footer), "footer states the GPLv3 license");
  check((await page.locator('footer a[href*="github.com"]').count()) >= 1, "footer links the source");
  check((await page.locator('footer #build').count()) === 1, "footer shows the build commit");
  // Cloudflare Web Analytics' cookie-free beacon is injected by the host on
  // the live site and is allowed (maintainer's choice); nothing else is.
  const thirdParty = await page.evaluate((origin) =>
    [...document.scripts].map((s) => s.src)
      .filter((src) => src && !src.startsWith(origin) && !src.startsWith("https://static.cloudflareinsights.com/")),
    new URL(base).origin);
  check(thirdParty.length === 0, `no third-party scripts (${thirdParty.join(", ")})`);
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  check(bg === "rgb(13, 13, 13)", `home background is CHARCOAL (${bg})`);

  // Console errors are counted on the home page only: the 404 check below
  // logs one on purpose.
  check(errors.length === 0, `no console errors on the home page (${errors.join(" | ")})`);

  // /play without its slash redirects.
  const bare = await page.request.get(at("/play"), { maxRedirects: 0 });
  const location = bare.headers()["location"] ?? "";
  check([301, 302, 307, 308].includes(bare.status()) && location.replace(/^https?:\/\/[^/]+/, "").startsWith("/play/"),
    `/play redirects to /play/ (${bare.status()} ${location})`);

  // Themed 404.
  const missing = await page.goto(at("/no-such-page-here/"));
  check(missing && missing.status() === 404, `unknown path returns 404 (${missing && missing.status()})`);
  check((await page.locator('a[href="/"]').count()) >= 1, "404 page links home");
  const bg404 = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  check(bg404 === "rgb(13, 13, 13)", `404 background is CHARCOAL (${bg404})`);

  // Phone width.
  const phone = await browser.newPage({ viewport: { width: 375, height: 740 } });
  await phone.goto(at("/"));
  const widths = await phone.evaluate(() => ({ doc: document.documentElement.scrollWidth, win: window.innerWidth }));
  check(widths.doc <= widths.win, `no horizontal scroll at 375 px (${widths.doc} > ${widths.win}?)`);
  for (const [name, selector] of ctas) {
    check(await phone.locator(selector).isVisible().catch(() => false), `"${name}" shown at 375 px`);
  }

  await browser.close();
  if (failures.length) {
    console.log(`\n${failures.length} check(s) failed`);
    process.exit(1);
  }
  console.log("\nsite checks passed");
})();
