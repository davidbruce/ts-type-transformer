// node run_wasm.mjs <impl.mjs> <glue.mjs> <module.wasm>
// Loads the JavaScript API, wires the generated glue, and calls `run`.
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';

const [impl, glue, wasm] = process.argv.slice(2);
// The API is globals the implementation sets, or the module it exports as
// `root`; with several packages, their modules as `modules`.
const { root, modules } = await import(pathToFileURL(impl));
const { makeEnv } = await import(pathToFileURL(glue));

let instance;
let passed = 0;
const failed = [];
// C++ is built for WASI: its few imports are not used by the tests.
const wasi = new Proxy({}, { get: () => () => 0 });
// Mistakes with handles are checked, and an exception no one took is a failure.
const options = { root, modules, check: true, unchecked: e => failed.push(`unchecked exception: ${e?.message ?? e}`) };
const env = { ...makeEnv(() => instance, options), check: (ok, line) => (ok ? passed++ : failed.push(line)) };
({ instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), { env, wasi_snapshot_preview1: wasi }));
instance.exports._initialize?.();
instance.exports.run();
// Asynchronous work settles, then `finish` checks what it did. A program
// whose work takes longer (a library's `init()`) exports `busy`, nonzero
// until it is done.
await new Promise(r => setTimeout(r, 50));
const deadline = Date.now() + 20000;
// A program waiting for a collection (a callback it dropped) gets one each time.
while (instance.exports.busy?.() && Date.now() < deadline) {
    globalThis.gc?.();
    await new Promise(r => setTimeout(r, 10));
}
instance.exports.finish?.();
if (failed.length || passed === 0) {
    console.error(`${failed.length} checks failed (test source lines: ${failed.join(', ')}); ${passed} passed`);
    process.exit(1);
}
console.log(`${passed} checks passed`);
