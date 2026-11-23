import * as pixi from 'pixi.js';
import { createRuntime, makeEnv as domEnv } from './glue/dom.mjs';
import { makeEnv as pixiEnv } from './glue/pixi_js.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const px = pixiEnv(() => instance, { runtime: rt, modules: { 'pixi.js': pixi } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  pixi_js: px.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
