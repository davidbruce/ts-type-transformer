import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runTargetBuild, spawnAsync } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));

function findWasiClangC() {
  if (process.env.WASI_SDK_PATH) {
    const p = path.join(process.env.WASI_SDK_PATH, 'bin/clang');
    if (fs.existsSync(p)) return p;
  }
  const dir = path.join(os.homedir(), '.wasi-sdk');
  if (fs.existsSync(dir)) {
    const entries = fs.readdirSync(dir).sort();
    while (entries.length > 0) {
      const p = path.join(dir, entries.pop(), 'bin/clang');
      if (fs.existsSync(p)) return p;
    }
  }
  throw new Error('WASI SDK clang not found (set WASI_SDK_PATH)');
}

const CLANG_WASM = [
  '--target=wasm32-wasip1',
  '-mexec-model=reactor',
  '-std=c23',
  '-O1',
  '-Wall',
  '-Wextra',
  '-Wno-unused-parameter',
  '-Werror',
  '-Wl,--export-table',
  '-Wl,--strip-debug',
];

let clangC = null;

await runTargetBuild({
  targetId: 'c',
  root: ROOT,
  domSentinel: 'dom.h',
  pkgSentinel: 'glue.mjs',
  beforeDemos: () => {
    clangC = findWasiClangC();
  },
  buildDemo: ({ demoDir, outPath, bindingsDir, buffered }) =>
    spawnAsync(
      clangC,
      [...CLANG_WASM, '-I', bindingsDir, '-o', outPath, path.join(demoDir, 'main.c')],
      { buffered },
    ),
});
