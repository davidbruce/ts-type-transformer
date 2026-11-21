import * as dateFns from 'date-fns';
import { createRuntime, makeEnv as domEnv } from '../../bindings/dom/glue.mjs';
import { makeEnv as dateFnsEnv } from '../../bindings/date_fns/glue.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const df = dateFnsEnv(() => instance, { runtime: rt, modules: { 'date-fns': dateFns } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  date_fns: df.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
