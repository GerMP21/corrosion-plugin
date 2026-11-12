// The AudioWorklet that runs Corrosion's engine (`add-web-app`). The main
// thread sends the wasm bytes; this processor compiles and instantiates them,
// renders each 128-frame quantum with `engine_process` and copies the
// channels out of wasm memory. Messages from the page: MIDI, parameter
// changes, tempo, presets. Messages back: level and voice count.

class CorrosionProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.e = null;
    this.engine = 0;
    this.queue = [];
    this.blocks = 0;
    this.sumSquares = 0;
    this.port.onmessage = (event) => this.onMessage(event.data);
  }

  onMessage(msg) {
    if (msg.type === "init") {
      WebAssembly.instantiate(msg.bytes, {}).then(({ instance }) => {
        this.e = instance.exports;
        this.engine = this.e.engine_new(sampleRate);
        for (const queued of this.queue) this.apply(queued);
        this.queue = [];
        this.port.postMessage({ type: "ready", params: this.e.engine_param_count(this.engine) });
      });
    } else if (this.e) {
      this.apply(msg);
    } else {
      this.queue.push(msg);
    }
  }

  writeBytes(bytes) {
    const ptr = this.e.engine_alloc(bytes.length);
    new Uint8Array(this.e.memory.buffer, ptr, bytes.length).set(bytes);
    return ptr;
  }

  apply(msg) {
    const { e, engine } = this;
    switch (msg.type) {
      case "midi":
        e.engine_midi(engine, msg.data[0], msg.data[1] ?? 0, msg.data[2] ?? 0);
        break;
      case "param":
        e.engine_set_param(engine, msg.index, msg.value);
        break;
      case "paramById": {
        const id = new TextEncoder().encode(msg.id);
        const index = e.engine_param_index(engine, this.writeBytes(id), id.length);
        if (index >= 0) e.engine_set_param(engine, index, msg.value);
        break;
      }
      case "tempo":
        e.engine_set_tempo(engine, msg.bpm);
        break;
      case "preset": {
        const json = new TextEncoder().encode(msg.json);
        const ok = e.engine_load_preset(engine, this.writeBytes(json), json.length);
        this.port.postMessage({ type: "presetLoaded", ok: ok === 1 });
        break;
      }
    }
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    if (!this.e || out.length < 2) return true;
    const frames = out[0].length;
    this.e.engine_process(this.engine, frames);
    const memory = this.e.memory.buffer;
    const left = new Float32Array(memory, this.e.engine_left(this.engine), frames);
    const right = new Float32Array(memory, this.e.engine_right(this.engine), frames);
    out[0].set(left);
    out[1].set(right);
    for (let i = 0; i < frames; i++) this.sumSquares += left[i] * left[i];
    if (++this.blocks % 64 === 0) {
      this.port.postMessage({
        type: "meter",
        rms: Math.sqrt(this.sumSquares / (64 * frames)),
        voices: this.e.engine_active_voices(this.engine),
      });
      this.sumSquares = 0;
    }
    return true;
  }
}

registerProcessor("corrosion", CorrosionProcessor);
