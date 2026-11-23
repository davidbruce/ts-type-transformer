import * as konva from 'konva';
import { createRuntime, makeEnv as domEnv } from './glue/dom.mjs';
import { makeEnv as konvaEnv } from './glue/konva.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const kv = konvaEnv(() => instance, { runtime: rt, modules: { konva } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  konva: kv.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
