// node run_fable.mjs <impl.mjs> <Test.mjs>
// Loads the JavaScript API, then the compiled F# test, which throws on failure.
import { pathToFileURL } from 'node:url';

const [impl, test] = process.argv.slice(2);
// The test reports last, perhaps after awaiting: one that never does has failed.
let reported = false;
const log = console.log;
console.log = (...args) => {
    reported ||= String(args[0]).includes('checks passed');
    log(...args);
};
process.on('exit', code => {
    if (code === 0 && !reported) {
        console.error('the test did not report: something it awaited never settled');
        process.exitCode = 1;
    }
});
// Fable's bindings name globals: a module's exports are made global.
const { root } = await import(pathToFileURL(impl));
if (root) Object.assign(globalThis, root);
await import(pathToFileURL(test));
