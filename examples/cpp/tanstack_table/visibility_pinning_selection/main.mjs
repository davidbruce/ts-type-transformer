import * as tableCore from '@tanstack/table-core';
import { createRuntime, makeEnv as domEnv } from '../../bindings/dom/glue.mjs';
import { makeEnv as tableEnv } from '../../bindings/tanstack_table_core/glue.mjs';

let instance;
const wasi = new Proxy({}, { get: () => () => 0 });
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const tt = tableEnv(() => instance, { runtime: rt, modules: { '@tanstack/table-core': tableCore } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  tanstack_table_core: tt.$pkg,
  wasi_snapshot_preview1: wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
