// node run_bench.mjs <impl.mjs> <bench.ts> <dir> <name>... 
// Times each workload of each program in <dir>/<name>/: a wasm module with
// its glue (bench.wasm, glue.mjs), or a JavaScript module (js/Bench.mjs).
// Every program must return the baseline's sums. Prints nanoseconds per
// iteration, best of five runs.
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [impl, baseline, dir, ...names] = process.argv.slice(2);
await import(pathToFileURL(impl));

const workloads = { calls: 2e6, objects: 3e5, properties: 1e6, strings_in: 5e5, strings_fresh: 5e5, strings_out: 3e5, callbacks: 1e6, compute: 2e7 };

async function load(name) {
    if (name === 'typescript') return import(pathToFileURL(baseline));
    const js = path.join(dir, name, 'js/Bench.mjs');
    if (fs.existsSync(js)) return import(pathToFileURL(js));
    const { makeEnv } = await import(pathToFileURL(path.join(dir, name, 'glue.mjs')));
    let instance;
    const wasi = new Proxy({}, { get: () => () => 0 });
    ({ instance } = await WebAssembly.instantiate(fs.readFileSync(path.join(dir, name, 'bench.wasm')), { env: makeEnv(() => instance), wasi_snapshot_preview1: wasi }));
    instance.exports._initialize?.();
    return instance.exports;
}

// Every program is loaded and warmed before any is timed, and the rounds
// are interleaved, so none is measured cold or first.
const programs = {};
for (const name of ['typescript', ...names]) programs[name] = await load(name);
for (const program of Object.values(programs)) {
    for (const [workload, n] of Object.entries(workloads)) program[workload](Math.ceil(n / 5));
}
const results = Object.fromEntries(Object.keys(programs).map(name => [name, {}]));
const sums = {};
const problems = new Set();
for (let round = 0; round < 5; round++) {
    for (const [workload, n] of Object.entries(workloads)) {
        for (const [name, program] of Object.entries(programs)) {
            const t = performance.now();
            const sum = program[workload](n);
            const ns = ((performance.now() - t) * 1e6) / n;
            results[name][workload] = Math.min(results[name][workload] ?? Infinity, ns);
            sums[workload] ??= sum;
            if (sum !== sums[workload]) problems.add(`${name} ${workload}: sum ${sum}, expected ${sums[workload]}`);
        }
    }
}

const cell = x => (x < 10 ? x.toFixed(1) : x.toFixed(0)).padStart(8);
console.log(`ns per iteration   ${Object.keys(results).map(n => n.padStart(10)).join('')}`);
for (const workload of Object.keys(workloads)) {
    console.log(`${workload.padEnd(19)}${Object.values(results).map(r => cell(r[workload]).padStart(10)).join('')}`);
}
if (problems.size) {
    console.error([...problems].join('\n'));
    process.exit(1);
}
