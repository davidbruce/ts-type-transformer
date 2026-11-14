// node run_wasm.mjs <impl.mjs> <glue.mjs> <module.wasm>
// Loads the JavaScript API, wires the generated glue, and calls `run`.
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const [impl, glue, wasm] = process.argv.slice(2);
await import(pathToFileURL(impl));
const { makeEnv } = await import(pathToFileURL(glue));

let instance;
let passed = 0;
const failed = [];
// C++ is built for WASI: its few imports are not used by the tests.
const wasi = new Proxy({}, { get: () => () => 0 });
const env = { ...makeEnv(() => instance), check: (ok, line) => (ok ? passed++ : failed.push(line)) };
({ instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), { env, wasi_snapshot_preview1: wasi }));
instance.exports._initialize?.();
instance.exports.run();
// Asynchronous work settles, then `finish` checks what it did.
await new Promise(r => setTimeout(r, 50));
instance.exports.finish?.();
if (failed.length || passed === 0) {
    console.error(`${failed.length} checks failed (test source lines: ${failed.join(', ')}); ${passed} passed`);
    process.exit(1);
}
console.log(`${passed} checks passed`);
