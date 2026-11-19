import * as fs from 'node:fs';
import * as path from 'node:path';
import * as events from 'node:events';
import { createRuntime, makeEnv as nodeEnv } from '../../bindings/node/glue.mjs';

let instance;
const wasi = new Proxy({}, { get: () => () => 0 });
const rt = createRuntime(() => instance);
const nodeMod = nodeEnv(() => instance, { runtime: rt, modules: { fs, path, events } });
const bytes = fs.readFileSync(new URL('./app.wasm', import.meta.url));
({ instance } = await WebAssembly.instantiate(bytes, {
  env: {
    ...rt.env,
    print(ptr, len) {
      process.stdout.write(new TextDecoder().decode(new Uint8Array(instance.exports.memory.buffer, ptr, len)));
    },
  },
  node: nodeMod.$pkg,
  wasi_snapshot_preview1: wasi,
}));
instance.exports._initialize?.();
instance.exports.start();
