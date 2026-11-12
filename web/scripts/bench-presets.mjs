// Cost of factory presets per 128-frame block in V8 with held chords
// (`add-web-app` task 1.3).
// Usage: node web/scripts/bench-presets.mjs engine.wasm preset...
import { readFileSync } from "node:fs";

const [wasmPath, ...presets] = process.argv.slice(2);
const wasm = new WebAssembly.Module(readFileSync(wasmPath));
const BLOCK = 128, RATE = 48000, BLOCKS = 400;
const budgetUs = (BLOCK / RATE) * 1e6;

function write(e, engine, bytes) {
  new Uint8Array(e.memory.buffer, e.engine_inbox(engine, bytes.length), bytes.length).set(bytes);
  return bytes.length;
}

console.log("voices: 1 / 4 / 8 / 16, us per block (% of the 2667 us budget)");
for (const path of presets) {
  const json = readFileSync(path);
  const cells = [];
  for (const voices of [1, 4, 8, 16]) {
    const e = new WebAssembly.Instance(wasm, {}).exports;
    const engine = e.engine_new(RATE);
    if (!e.engine_load_preset(engine, write(e, engine, json))) throw new Error(`bad preset ${path}`);
    for (let v = 0; v < voices; v++) e.engine_midi(engine, 0x90, 36 + v * 3, 100);
    for (let i = 0; i < 20; i++) e.engine_process(engine, BLOCK);
    const t0 = performance.now();
    for (let i = 0; i < BLOCKS; i++) e.engine_process(engine, BLOCK);
    const us = ((performance.now() - t0) * 1000) / BLOCKS;
    cells.push(`${us.toFixed(0)} (${(100 * us / budgetUs).toFixed(0)} %)`);
  }
  console.log(`${path.split("/").slice(-2).join("/").padEnd(48)} ${cells.join("  ")}`);
}
