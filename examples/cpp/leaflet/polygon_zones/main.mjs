import * as leaflet from 'leaflet';
import { createRuntime, makeEnv as domEnv } from './glue/dom.mjs';
import { makeEnv as leafletEnv } from './glue/leaflet.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const lf = leafletEnv(() => instance, { runtime: rt, modules: { leaflet } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  leaflet: lf.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
