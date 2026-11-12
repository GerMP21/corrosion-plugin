// Corrosion's web host (`add-web-app`). The editor (eframe in a canvas)
// runs on the main thread, the engine in an AudioWorklet, and this page
// plays the DAW between them:
// - editor command batches go to the worklet, engine reports come back;
// - Web MIDI and the BPM field go straight to the engine;
// - the session autosaves to localStorage, user presets live in IndexedDB,
//   preset files can be opened and downloaded.

import initEditor, * as editor from "./corrosion_web_editor.js";

const $ = (id) => document.getElementById(id);
const status = $("status");
const loadLabel = $("load");
const SESSION_KEY = "corrosion.session";
const BPM_KEY = "corrosion.bpm";
const LOAD_WARNING = 0.8;

let node = null;
/** Batches the editor sent before audio started, in order. */
const pending = [];

function post(message, transfer = []) {
  node?.port.postMessage(message, transfer);
}

function postBatch(bytes) {
  // Copy: the view points into the editor's wasm memory.
  const copy = bytes.slice();
  if (node) post({ type: "batch", bytes: copy.buffer }, [copy.buffer]);
  else pending.push(copy);
}

// ---------------------------------------------------------------- storage

/** localStorage that may be unavailable (private windows, blocked storage). */
const local = {
  get(key) { try { return localStorage.getItem(key); } catch { return null; } },
  set(key, value) { try { localStorage.setItem(key, value); } catch { /* not kept */ } },
};

/** User presets in IndexedDB: `{key, folder, file, json}`. */
const presetDb = (() => {
  let db = null;
  const open = () => new Promise((resolve, reject) => {
    const request = indexedDB.open("corrosion", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("presets", { keyPath: "key" });
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  const store = async (mode) => {
    db ??= await open();
    return db.transaction("presets", mode).objectStore("presets");
  };
  const done = (request) => new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
  return {
    async all() { return done((await store("readonly")).getAll()); },
    async put(preset) {
      const key = `${preset.folder}/${preset.file}`.toLowerCase();
      return done((await store("readwrite")).put({ key, ...preset }));
    },
  };
})();

async function loadUserPresets() {
  try {
    const stored = await presetDb.all();
    editor.set_user_presets(JSON.stringify(stored.map(({ folder, file, json }) => ({ folder, file, json }))));
  } catch (error) {
    console.warn("User presets unavailable:", error);
  }
}

async function persistSavedPresets() {
  const saved = JSON.parse(editor.take_saved_presets());
  for (const preset of saved) {
    try { await presetDb.put(preset); } catch (error) { console.warn("Couldn't keep a preset:", error); }
  }
}

let lastSession = null;
function autosave() {
  const session = editor.current_preset_json("Session");
  if (session && session !== lastSession) {
    local.set(SESSION_KEY, session);
    lastSession = session;
  }
}

// ---------------------------------------------------------------- audio

async function startAudio() {
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
      if (msg.type === "meter") onMeter(context, msg);
    };
  });
  // The worklet thread only runs once the context does.
  await context.resume();
  worklet.port.postMessage({ type: "init", bytes }, [bytes]);
  await ready;
  node = worklet;
  post({ type: "tempo", bpm: Number($("bpm").value) || 120 });
  // Everything the editor changed so far: its state is the engine's
  // default plus these batches.
  for (const batch of pending.splice(0)) post({ type: "batch", bytes: batch.buffer }, [batch.buffer]);
  status.textContent = `Audio on · ${context.sampleRate} Hz`;
  await setUpMidi();
  return context;
}

function onMeter(context, msg) {
  window.corrosionMeter = msg;
  editor.push_report(new Uint8Array(msg.report));
  const voices = `${msg.voices} ${msg.voices === 1 ? "voice" : "voices"}`;
  status.textContent = `Audio on · ${context.sampleRate} Hz · ${voices}`;
  const underruns = context.playbackStats?.underrunEvents;
  const load = Math.round(msg.load * 100);
  loadLabel.textContent = `CPU ${load} %` + (underruns ? ` · ${underruns} dropouts` : "");
  loadLabel.classList.toggle("warn", msg.load > LOAD_WARNING);
  loadLabel.title = msg.load > LOAD_WARNING
    ? "The browser is close to its real-time limit: play fewer notes or pick a lighter preset."
    : "Share of each audio block spent rendering.";
}

// ---------------------------------------------------------------- MIDI

async function setUpMidi() {
  const select = $("midi");
  select.replaceChildren();
  if (!navigator.requestMIDIAccess) {
    select.append(new Option("Not available in this browser"));
    select.title = "This browser has no Web MIDI. The on-screen and computer keyboards still play.";
    return;
  }
  let access;
  try {
    access = await navigator.requestMIDIAccess();
  } catch {
    select.append(new Option("Not allowed"));
    select.title = "MIDI access was refused. The on-screen and computer keyboards still play.";
    return;
  }
  const onMessage = (event) => {
    const chosen = select.value;
    if (chosen !== "all" && chosen !== event.currentTarget.id) return;
    const [status, data1 = 0, data2 = 0] = event.data;
    if (status >= 0xf0) return; // clock, sysex, active sensing
    post({ type: "midi", data: [status, data1, data2] });
  };
  const refresh = () => {
    const previous = select.value || "all";
    select.replaceChildren(new Option("All inputs", "all"), new Option("None", "none"));
    for (const input of access.inputs.values()) {
      select.append(new Option(input.name, input.id));
      input.onmidimessage = onMessage;
    }
    select.value = [...select.options].some((o) => o.value === previous) ? previous : "all";
    select.disabled = false;
  };
  access.onstatechange = refresh;
  refresh();
}

// ---------------------------------------------------------------- controls

$("start").addEventListener("click", async (event) => {
  const button = event.currentTarget;
  button.disabled = true;
  try {
    await startAudio();
    button.textContent = "Audio on";
    $("editor").focus();
  } catch (error) {
    status.textContent = `Could not start audio: ${error.message}`;
    button.disabled = false;
  }
});

const bpm = $("bpm");
bpm.value = local.get(BPM_KEY) ?? bpm.value;
bpm.addEventListener("change", () => {
  const value = Math.min(300, Math.max(20, Number(bpm.value) || 120));
  bpm.value = value;
  local.set(BPM_KEY, String(value));
  post({ type: "tempo", bpm: value });
});

$("open").addEventListener("click", () => $("file").click());
$("file").addEventListener("change", async (event) => {
  const file = event.currentTarget.files[0];
  event.currentTarget.value = "";
  if (!file) return;
  try {
    editor.open_preset_file(await file.text());
    await persistSavedPresets();
  } catch (error) {
    status.textContent = String(error);
  }
});

$("download").addEventListener("click", () => {
  const name = prompt("Preset name", "My Corrosion sound");
  if (!name) return;
  const json = editor.current_preset_json(name);
  const safe = name.replace(/[\\/:*?"<>|]/g, "_").trim() || "Untitled";
  const link = document.createElement("a");
  link.href = URL.createObjectURL(new Blob([json], { type: "application/json" }));
  link.download = `${safe}.corrosion-preset`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
});

// Held keys would hang when the page loses focus or is hidden.
window.addEventListener("blur", () => editor.release_keys());
document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    editor.release_keys();
    autosave();
  }
});
window.addEventListener("pagehide", autosave);

// ---------------------------------------------------------------- start

await initEditor();
const seed = (crypto.getRandomValues(new Uint32Array(1))[0] | 1) >>> 0;
await editor.start($("editor"), postBatch, seed, local.get(SESSION_KEY) ?? undefined);
await loadUserPresets();
setInterval(() => { autosave(); persistSavedPresets(); }, 1500);
// For the page's own checks (web/scripts/features.cjs).
window.corrosionPost = post;
window.corrosionEditorReady = true;
