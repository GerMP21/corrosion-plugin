// Minimal static server for web/dist that follows the two Workers
// static-asset rules the site relies on (`add-site-shell`):
// html_handling = "auto-trailing-slash" (a directory without its slash
// redirects to it, `/x/` serves `/x/index.html`) and
// not_found_handling = "404-page" (unknown paths get the nearest 404.html
// with status 404). CI uses it; locally `wrangler dev` works too.
// Usage: node web/scripts/serve.cjs web/dist 8787
const http = require("http");
const fs = require("fs");
const path = require("path");

const root = path.resolve(process.argv[2] ?? "dist");
const port = Number(process.argv[3] ?? 8787);
const types = {
  ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css",
  ".wasm": "application/wasm", ".json": "application/json", ".svg": "image/svg+xml",
  ".png": "image/png", ".webp": "image/webp", ".opus": "audio/ogg", ".ogg": "audio/ogg",
  ".woff2": "font/woff2", ".txt": "text/plain",
};
const isFile = (p) => { try { return fs.statSync(p).isFile(); } catch { return false; } };
const isDir = (p) => { try { return fs.statSync(p).isDirectory(); } catch { return false; } };

function send(res, status, file) {
  res.writeHead(status, { "Content-Type": types[path.extname(file)] ?? "application/octet-stream" });
  fs.createReadStream(file).pipe(res);
}

function notFound(res, urlPath) {
  // The nearest 404.html, from the requested directory up to the root.
  let dir = path.join(root, path.dirname(urlPath));
  while (dir.startsWith(root)) {
    const page = path.join(dir, "404.html");
    if (isFile(page)) return send(res, 404, page);
    if (dir === root) break;
    dir = path.dirname(dir);
  }
  res.writeHead(404, { "Content-Type": "text/plain" });
  res.end("not found");
}

http.createServer((req, res) => {
  const url = new URL(req.url, "http://localhost");
  const urlPath = decodeURIComponent(url.pathname);
  const file = path.join(root, urlPath);
  if (!file.startsWith(root)) return notFound(res, "/");
  if (urlPath.endsWith("/")) {
    const index = path.join(file, "index.html");
    return isFile(index) ? send(res, 200, index) : notFound(res, urlPath);
  }
  if (isFile(file)) return send(res, 200, file);
  if (isDir(file) && isFile(path.join(file, "index.html"))) {
    res.writeHead(307, { Location: urlPath + "/" + url.search });
    return res.end();
  }
  if (isFile(file + ".html")) return send(res, 200, file + ".html");
  notFound(res, urlPath);
}).listen(port, "127.0.0.1", () => console.log(`serving ${root} on http://127.0.0.1:${port}/`));
