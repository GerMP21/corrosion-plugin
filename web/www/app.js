// Corrosion's web host (`add-web-app`): the editor (eframe in a canvas) on
// the main thread, the engine in an AudioWorklet, and the page between
// them. The editor's command batches are posted to the worklet; the
// worklet's reports go back to the editor.

import initEditor, * as editor from "./corrosion_web_editor.js";

const status = document.getElementById("status");
const startButton = document.getElementById("start");
let node = null;
/** Batches the editor sent before audio started, in order. */
const pending = [];

function postBatch(bytes) {
  // Copy: the view points into the editor's wasm memory.
  const copy = bytes.slice();
  if (node) node.port.postMessage({ type: "batch", bytes: copy.buffer }, [copy.buffer]);
  else pending.push(copy);
}

export async function startAudio() {
  const context = new AudioContext({ latencyHint: "interactive" });
  // The worklet gets the wasm bytes and compiles them itself: Chromium
  // silently drops a `WebAssembly.Module` posted to a worklet port.
  const [bytes] = await Promise.all([
    fetch("corrosion_web_engine.wasm").then((response) => response.arrayBuffer()),
    context.audioWorklet.addModule("engine-worklet.js"),
  ]);
  const worklet = new AudioWorkletNode(context, "corrosion", { numberOfInputs: 0, outputChannelCount: [2] });
  worklet.connect(context.destination);
  const ready = new Promise((resolve) => {
    worklet.port.onmessage = (event) => {
      const msg = event.data;
      if (msg.type === "ready") resolve(msg);
      if (msg.type === "meter") {
        window.corrosionMeter = msg;
        editor.push_report(new Uint8Array(msg.report));
        status.textContent = `Audio on · ${context.sampleRate} Hz · ${msg.voices} ${msg.voices === 1 ? "voice" : "voices"}`;
      }
    };
  });
  // The worklet thread only runs once the context does.
  await context.resume();
  worklet.port.postMessage({ type: "init", bytes }, [bytes]);
  await ready;
  node = worklet;
  // Everything the editor changed so far (its state is the engine's
  // default plus these batches).
  for (const batch of pending.splice(0)) {
    node.port.postMessage({ type: "batch", bytes: batch.buffer }, [batch.buffer]);
  }
  status.textContent = `Audio on · ${context.sampleRate} Hz`;
  return context;
}

startButton.addEventListener("click", async () => {
  startButton.disabled = true;
  try {
    await startAudio();
    startButton.textContent = "Audio on";
    document.getElementById("editor").focus();
  } catch (error) {
    status.textContent = `Could not start audio: ${error.message}`;
    startButton.disabled = false;
  }
});

// Held keys would hang when the page loses focus or is hidden.
window.addEventListener("blur", () => editor.release_keys());
document.addEventListener("visibilitychange", () => document.hidden && editor.release_keys());

await initEditor();
const seed = (crypto.getRandomValues(new Uint32Array(1))[0] | 1) >>> 0;
await editor.start(document.getElementById("editor"), postBatch, seed);
window.corrosionEditorReady = true;
