import * as three from 'three';
import { createRuntime, makeEnv as domEnv } from './glue/dom.mjs';
import { makeEnv as threeEnv } from './glue/three.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const th = threeEnv(() => instance, { runtime: rt, modules: { three } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  three: th.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
