// Cost of the web engine per 128-frame block in V8 (Chrome's engine), by
// quality mode and voice count (`add-web-app` task 1.3).
// Usage: node web/scripts/bench-engine.mjs path/to/corrosion_web_engine.wasm
import { readFileSync } from "node:fs";

const wasm = new WebAssembly.Module(readFileSync(process.argv[2]));
const BLOCK = 128, RATE = 48000, BLOCKS = 1500;
const budgetUs = (BLOCK / RATE) * 1e6;
const qualities = ["Eco", "Normal", "High", "Render"];

function paramIndex(e, engine, id) {
  const bytes = new TextEncoder().encode(id);
  const ptr = e.engine_alloc(bytes.length);
  new Uint8Array(e.memory.buffer, ptr, bytes.length).set(bytes);
  return e.engine_param_index(engine, ptr, bytes.length);
}

console.log(`budget ${budgetUs.toFixed(0)} us per block`);
console.log("quality  voices  us/block  % budget  voices sounding at end");
for (let q = 0; q < 4; q++) {
  for (const voices of [1, 4, 8, 16]) {
    const e = new WebAssembly.Instance(wasm, {}).exports;
    const engine = e.engine_new(RATE);
    e.engine_set_param(engine, paramIndex(e, engine, "quality_mode"), q / 3);
    for (let v = 0; v < voices; v++) e.engine_midi(engine, 0x90, 36 + v * 3, 100);
    for (let i = 0; i < 20; i++) e.engine_process(engine, BLOCK);
    const t0 = performance.now();
    for (let i = 0; i < BLOCKS; i++) e.engine_process(engine, BLOCK);
    const us = ((performance.now() - t0) * 1000) / BLOCKS;
    console.log(`${qualities[q].padEnd(8)} ${String(voices).padStart(6)}  ${us.toFixed(0).padStart(8)}  ${(100 * us / budgetUs).toFixed(0).padStart(7)} %  ${e.engine_active_voices(engine)}`);
  }
}
