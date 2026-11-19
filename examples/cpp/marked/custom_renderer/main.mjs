import * as marked from 'marked';
import { createRuntime, makeEnv as domEnv } from '../../bindings/dom/glue.mjs';
import { makeEnv as markedEnv } from '../../bindings/marked/glue.mjs';

let instance;
const wasi = new Proxy({}, { get: () => () => 0 });
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const m = markedEnv(() => instance, { runtime: rt, modules: { marked } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  marked: m.$pkg,
  wasi_snapshot_preview1: wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
