// Corrosion's web host (`add-web-app`): starts the AudioContext on a click,
// loads the engine into the AudioWorklet and sends it notes.

const status = document.getElementById("status");
const noteButton = document.getElementById("note");
let node = null;

export async function start() {
  const context = new AudioContext({ latencyHint: "interactive" });
  // The worklet gets the wasm bytes and compiles them itself: Chromium
  // silently drops a `WebAssembly.Module` posted to a worklet port.
  const [bytes] = await Promise.all([
    fetch("corrosion_web_engine.wasm").then((response) => response.arrayBuffer()),
    context.audioWorklet.addModule("engine-worklet.js"),
  ]);
  node = new AudioWorkletNode(context, "corrosion", { numberOfInputs: 0, outputChannelCount: [2] });
  node.connect(context.destination);
  const ready = new Promise((resolve) => {
    node.port.onmessage = (event) => {
      const msg = event.data;
      if (msg.type === "ready") resolve(msg);
      if (msg.type === "meter") {
        window.corrosionMeter = msg;
        status.textContent = `${context.sampleRate} Hz · ${msg.voices} voices · level ${msg.rms.toFixed(4)}`;
      }
    };
  });
  // The worklet thread only runs once the context does.
  await context.resume();
  node.port.postMessage({ type: "init", bytes }, [bytes]);
  await ready;
  noteButton.disabled = false;
  status.textContent = `Audio on at ${context.sampleRate} Hz.`;
  return context;
}

export function midi(data) {
  node?.port.postMessage({ type: "midi", data });
}

document.getElementById("start").addEventListener("click", async (event) => {
  event.target.disabled = true;
  try {
    await start();
  } catch (error) {
    status.textContent = `Could not start audio: ${error.message}`;
    event.target.disabled = false;
  }
});
noteButton.addEventListener("pointerdown", () => midi([0x90, 60, 100]));
noteButton.addEventListener("pointerup", () => midi([0x80, 60, 0]));
noteButton.addEventListener("pointerleave", () => midi([0x80, 60, 0]));
