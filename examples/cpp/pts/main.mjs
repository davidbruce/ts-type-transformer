import * as pts from 'pts';
import { createRuntime, makeEnv as ptsEnv } from './bindings/pts/glue.mjs';

let instance;
const wasi = new Proxy({}, { get: () => () => 0 });
const rt = createRuntime(() => instance);
const p = ptsEnv(() => instance, { runtime: rt, modules: { pts } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  pts: p.$pkg,
  wasi_snapshot_preview1: wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
