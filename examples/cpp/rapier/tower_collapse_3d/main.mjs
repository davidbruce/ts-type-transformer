import * as babylonLite from '@babylonjs/lite';
import * as rapier from '@dimforge/rapier3d-compat';
import { createRuntime, makeEnv as domEnv } from './glue/dom.mjs';
import { makeEnv as babylonEnv } from './glue/babylonjs_lite.mjs';
import { makeEnv as rapierEnv } from './glue/dimforge_rapier3d_compat.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const babylon = babylonEnv(() => instance, { runtime: rt, modules: { '@babylonjs/lite': babylonLite } });
const rp = rapierEnv(() => instance, { runtime: rt, modules: { '@dimforge/rapier3d-compat': rapier } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  babylonjs_lite: babylon.$pkg,
  dimforge_rapier3d_compat: rp.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
