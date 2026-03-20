import init, { validate } from '/wasm/horn_wasm.js';

let ready = init();

self.onmessage = async function(e) {
    await ready;
    const { id, name, data } = e.data;
    const t0 = performance.now();
    const report = validate(name, data);
    const ms = (performance.now() - t0).toFixed(1);
    self.postMessage({ id, report, ms, name });
};
