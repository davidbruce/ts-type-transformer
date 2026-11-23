import * as pmModel from 'prosemirror-model';
import * as pmTransform from 'prosemirror-transform';
import * as pmState from 'prosemirror-state';
import * as pmView from 'prosemirror-view';
import { createRuntime, makeEnv as domEnv } from './glue/dom.mjs';
import { makeEnv as pmEnv } from './glue/prosemirror.mjs';

let instance;
const rt = createRuntime(() => instance);
const dom = domEnv(() => instance, { runtime: rt });
const pm = pmEnv(() => instance, {
  runtime: rt,
  modules: {
    'prosemirror-model': pmModel,
    'prosemirror-transform': pmTransform,
    'prosemirror-state': pmState,
    'prosemirror-view': pmView,
  },
});
const bytes = await (await fetch('./app.wasm')).arrayBuffer();
({ instance } = await WebAssembly.instantiate(bytes, {
  env: rt.env,
  dom: dom.$pkg,
  prosemirror: pm.$pkg,
  wasi_snapshot_preview1: rt.wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
