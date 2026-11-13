// node run_fable.mjs <impl.mjs> <Test.mjs>
// Loads the JavaScript API, then the compiled F# test, which throws on failure.
import { pathToFileURL } from 'node:url';

const [impl, test] = process.argv.slice(2);
await import(pathToFileURL(impl));
await import(pathToFileURL(test));
