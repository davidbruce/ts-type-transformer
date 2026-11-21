import * as echarts from 'echarts';
import { createRuntime, makeEnv as domEnv } from '../../bindings/dom/glue.mjs';
import { makeEnv as echartsEnv } from '../../bindings/echarts/glue.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const ec = echartsEnv(() => instance, { runtime: rt, modules: { echarts } });
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  echarts: ec.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
