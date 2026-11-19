import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, 'examples.json'), 'utf8'));
const rawArgs = process.argv.slice(2);
const force = rawArgs.includes('--force');
const filters = rawArgs.filter(a => a !== '--force');

function isUpToDate(ex, outputPath) {
  if (force || !fs.existsSync(outputPath)) return false;
  const srcPath = path.resolve(ROOT, ex.source);
  if (!fs.existsSync(srcPath)) return false;
  return fs.statSync(outputPath).mtimeMs >= fs.statSync(srcPath).mtimeMs;
}

const PACKAGES = {
  pts: { npm: ['pts'], needsDom: false, outDir: 'pts', name: 'pts' },
  babylon_lite: { npm: ['@babylonjs/lite'], needsDom: true, outDir: 'babylonjs_lite', name: 'babylonjs_lite' },
  katex: { npm: ['katex'], needsDom: true, outDir: 'katex', name: 'katex' },
  marked: { npm: ['marked'], needsDom: true, outDir: 'marked', name: 'marked' },
  date_fns: { npm: ['date-fns'], needsDom: false, outDir: 'date_fns', name: 'date_fns' },
  node: { npm: ['fs', 'path', 'events'], needsDom: false, outDir: 'node', name: 'node' },
  leaflet: { npm: ['leaflet'], needsDom: true, outDir: 'leaflet', name: 'leaflet' },
  pixi: { npm: ['pixi.js'], needsDom: true, outDir: 'pixi_js', name: 'pixi_js' },
  echarts: { npm: ['echarts'], needsDom: true, outDir: 'echarts', name: 'echarts' },
  konva: { npm: ['konva'], needsDom: true, outDir: 'konva', name: 'konva' },
  three: { npm: ['three'], needsDom: true, outDir: 'three', name: 'three' },
  rapier: { npm: ['@dimforge/rapier3d-compat'], needsDom: false, outDir: 'dimforge_rapier3d_compat', name: 'dimforge_rapier3d_compat' },
  tanstack_table: { npm: ['@tanstack/table-core'], needsDom: false, outDir: 'tanstack_table_core', name: 'tanstack_table_core' },
  prosemirror: { npm: ['prosemirror-model', 'prosemirror-transform', 'prosemirror-state', 'prosemirror-view'], needsDom: true, outDir: 'prosemirror', name: 'prosemirror' },
};

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

const clangxx = findWasiClang();
const CLANGXX_WASM = ['--target=wasm32-wasip1', '-mexec-model=reactor', '-std=c++20', '-fno-exceptions', '-O1', '-Wall', '-Werror', '-Wl,--export-table', '-Wl,--strip-debug'];
const CLANGXX_OBJ = CLANGXX_WASM.filter(f => !f.startsWith('-Wl,') && !f.startsWith('-mexec-model'));

const tsrsProto =
  process.env.TSRS_PROTO ??
  ([path.resolve(ROOT, '../../target/debug/tsrs_proto'), path.resolve(ROOT, '../../target/release/tsrs_proto')]
    .filter(p => fs.existsSync(p))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)[0] || 'tsrs_proto');
const cfg = path.resolve(ROOT, '../targets/cpp.toml');
const domCfg = path.resolve(ROOT, '../modules/dom.toml');
const bindingsDir = path.join(ROOT, 'bindings');
const objRoot = path.join(bindingsDir, '.obj');
const fallbackFrom = path.resolve(ROOT, '../../target/libraries');
const fromDir = fs.existsSync(path.join(ROOT, 'node_modules/pts')) ? ROOT : fallbackFrom;

const selected = manifest.examples.filter(
  e => filters.length === 0 || filters.some(f => e.package === f || e.id === f || e.id.startsWith(`${f}/`)),
);
const neededPkgs = [...new Set(selected.map(e => e.package))];
const needsDom = neededPkgs.some(p => PACKAGES[p]?.needsDom);

fs.mkdirSync(bindingsDir, { recursive: true });
if (needsDom && !fs.existsSync(path.join(bindingsDir, 'dom/dom.hpp'))) {
  execFileSync(tsrsProto, ['lib.dom.d.ts', '--config', cfg, '--config', domCfg, '--emit', 'cpp', '--name', 'dom', '--out', bindingsDir], {
    cwd: ROOT,
    stdio: 'inherit',
  });
}

function ensurePkgObjects(pkgDirName) {
  const pkgObjDir = path.join(objRoot, pkgDirName);
  const doneFile = path.join(pkgObjDir, '.done');
  const srcs = collectCpp(path.join(bindingsDir, pkgDirName));
  const objs = srcs.map((_, i) => path.join(pkgObjDir, `m${i}.o`));
  if (!fs.existsSync(doneFile)) {
    fs.mkdirSync(pkgObjDir, { recursive: true });
    srcs.forEach((src, i) => {
      execFileSync(clangxx, [...CLANGXX_OBJ, '-c', '-I', bindingsDir, src, '-o', objs[i]], { stdio: 'inherit' });
    });
    fs.writeFileSync(doneFile, 'ok');
  }
  return objs;
}

for (const pkg of neededPkgs) {
  const spec = PACKAGES[pkg];
  if (!spec) throw new Error(`Unknown package spec: ${pkg}`);
  if (!fs.existsSync(path.join(bindingsDir, spec.outDir))) {
    const args = ['--from', fromDir];
    for (const n of spec.npm) args.push('--package', n);
    args.push('--config', cfg, '--emit', 'cpp');
    if (spec.npm.length > 1) args.push('--name', spec.name);
    if (spec.needsDom) args.push('--ref', path.join(bindingsDir, 'dom'));
    args.push('--out', bindingsDir);
    execFileSync(tsrsProto, args, { cwd: ROOT, stdio: 'inherit' });
  }
}

let domObjs = null;
for (const ex of selected) {
  const demoDir = path.join(ROOT, path.dirname(ex.path));
  const outPath = path.join(demoDir, 'app.wasm');
  if (isUpToDate(ex, outPath)) continue;
  const spec = PACKAGES[ex.package];
  if (spec.needsDom && domObjs === null) domObjs = ensurePkgObjects('dom');
  const objs = [...(spec.needsDom ? domObjs : []), ...ensurePkgObjects(spec.outDir)];
  console.log(`Building cpp ${ex.id}...`);
  execFileSync(clangxx, [...CLANGXX_WASM, '-I', bindingsDir, '-o', outPath, path.join(demoDir, 'main.cpp'), ...objs], {
    stdio: 'inherit',
  });
}
