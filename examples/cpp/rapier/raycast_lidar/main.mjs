import * as rapier from '@dimforge/rapier3d-compat';
import { createRuntime, makeEnv as domEnv } from '../../bindings/dom/glue.mjs';
import { makeEnv as rapierEnv } from '../../bindings/dimforge_rapier3d_compat/glue.mjs';

let instance;
const wasi = new Proxy({}, { get: () => () => 0 });
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const rp = rapierEnv(() => instance, { runtime: rt, modules: { '@dimforge/rapier3d-compat': rapier } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  dimforge_rapier3d_compat: rp.$pkg,
  wasi_snapshot_preview1: wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
