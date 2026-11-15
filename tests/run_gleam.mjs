// node run_gleam.mjs <impl.mjs> <check.mjs>
// Loads the JavaScript API, then runs the compiled Gleam test's `main`, which
// panics on failure and reports last, perhaps after awaiting.
import { pathToFileURL } from 'node:url';

const [impl, test] = process.argv.slice(2);
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
await import(pathToFileURL(impl));
const { main } = await import(pathToFileURL(test));
main();
