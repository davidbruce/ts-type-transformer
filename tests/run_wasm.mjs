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
const env = { ...makeEnv(() => instance.exports.memory), check: (ok, line) => (ok ? passed++ : failed.push(line)) };
({ instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), { env }));
instance.exports.run();
if (failed.length || passed === 0) {
    console.error(`${failed.length} checks failed (test source lines: ${failed.join(', ')}); ${passed} passed`);
    process.exit(1);
}
console.log(`${passed} checks passed`);
