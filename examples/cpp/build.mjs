import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { latestMtimeRecursive, runPool, runTargetBuild, spawnAsync } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));

function findWasiClang() {
  if (process.env.WASI_SDK_PATH) {
    const p = path.join(process.env.WASI_SDK_PATH, 'bin/clang++');
    if (fs.existsSync(p)) return p;
  }
  const dir = path.join(os.homedir(), '.wasi-sdk');
  if (fs.existsSync(dir)) {
    const entries = fs.readdirSync(dir).sort();
    while (entries.length > 0) {
      const p = path.join(dir, entries.pop(), 'bin/clang++');
      if (fs.existsSync(p)) return p;
    }
  }
  throw new Error('WASI SDK clang++ not found (set WASI_SDK_PATH)');
}

function collectCpp(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, e.name);
    if (e.isDirectory()) collectCpp(full, out);
    else if (e.name.endsWith('.cpp')) out.push(full);
  }
  return out.sort();
}

const CLANGXX_WASM = [
  '--target=wasm32-wasip1',
  '-mexec-model=reactor',
  '-std=c++20',
  '-fno-exceptions',
  '-O1',
  '-Wall',
  '-Wextra',
  '-Wno-unused-parameter',
  '-Werror',
  '-Wl,--export-table',
  '-Wl,--strip-debug',
];
const CLANGXX_OBJ = CLANGXX_WASM.filter(f => !f.startsWith('-Wl,') && !f.startsWith('-mexec-model'));

let clangxx = null;
const objCache = new Map();

async function ensurePkgObjects(pkgDirName, bindingsDir, objRoot, jobs, force) {
  if (objCache.has(pkgDirName)) return objCache.get(pkgDirName);
  const pkgSrcDir = path.join(bindingsDir, pkgDirName);
  const pkgObjDir = path.join(objRoot, pkgDirName);
  const doneFile = path.join(pkgObjDir, '.done');
  const srcs = collectCpp(pkgSrcDir);
  const objs = srcs.map((_, i) => path.join(pkgObjDir, `m${i}.o`));
  const upToDate =
    !force &&
    fs.existsSync(doneFile) &&
    latestMtimeRecursive(pkgSrcDir) <= fs.statSync(doneFile).mtimeMs;

  if (!upToDate) {
    fs.rmSync(pkgObjDir, { recursive: true, force: true });
    fs.mkdirSync(pkgObjDir, { recursive: true });
    await runPool(
      srcs.map((src, i) => ({ src, obj: objs[i] })),
      jobs,
      ({ src, obj }) =>
        spawnAsync(clangxx, [...CLANGXX_OBJ, '-c', '-I', bindingsDir, src, '-o', obj], {
          buffered: jobs > 1,
        }),
    );
    fs.writeFileSync(doneFile, 'ok');
  }

  objCache.set(pkgDirName, objs);
  return objs;
}

await runTargetBuild({
  targetId: 'cpp',
  root: ROOT,
  domSentinel: 'dom.hpp',
  pkgSentinel: 'glue.mjs',
  onRegenerate: (dirName, bindingsDir) => {
    fs.rmSync(path.join(bindingsDir, '.obj', dirName), { recursive: true, force: true });
  },
  beforeDemos: async ({ buildPkgs, buildDom, packages, bindingsDir, jobs, force }) => {
    clangxx = findWasiClang();
    const objRoot = path.join(bindingsDir, '.obj');
    if (buildDom) {
      await ensurePkgObjects('dom', bindingsDir, objRoot, jobs, force);
    }
    const pkgDirs = [...new Set(buildPkgs.map(p => packages[p].outDir))];
    for (const dirName of pkgDirs) {
      await ensurePkgObjects(dirName, bindingsDir, objRoot, jobs, force);
    }
  },
  buildDemo: ({ demoDir, outPath, specs, bindingsDir, buffered }) => {
    const seen = new Set();
    const objs = [];
    if (specs.some(s => s.needsDom)) {
      objs.push(...(objCache.get('dom') ?? []));
    }
    for (const s of specs) {
      if (seen.has(s.outDir)) continue;
      seen.add(s.outDir);
      objs.push(...(objCache.get(s.outDir) ?? []));
    }
    return spawnAsync(
      clangxx,
      [...CLANGXX_WASM, '-I', bindingsDir, '-o', outPath, path.join(demoDir, 'main.cpp'), ...objs],
      { buffered },
    );
  },
});
