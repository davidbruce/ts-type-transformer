import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, 'examples.json'), 'utf8'));
const filters = process.argv.slice(2);

const PACKAGES = {
  pts: { npm: ['pts'], needsDom: false, outDir: 'pts', name: 'pts' },
  babylon_lite: { npm: ['@babylonjs/lite'], needsDom: true, outDir: 'babylonjs_lite', name: 'babylonjs_lite' },
  katex: { npm: ['katex'], needsDom: true, outDir: 'katex', name: 'katex' },
  marked: { npm: ['marked'], needsDom: false, outDir: 'marked', name: 'marked' },
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

const CLANG_WASM = [
  '--target=wasm32',
  '-std=c11',
  '-pedantic',
  '-nostdlib',
  '-ffreestanding',
  '-O1',
  '-Wall',
  '-Werror',
  '-Wl,--no-entry',
  '-Wl,--allow-undefined',
  '-Wl,--export-table',
];

const tsrsProto =
  process.env.TSRS_PROTO ??
  ([path.resolve(ROOT, '../../target/release/tsrs_proto'), path.resolve(ROOT, '../../target/debug/tsrs_proto')].find(p =>
    fs.existsSync(p),
  ) || 'tsrs_proto');
const cfg = path.resolve(ROOT, '../targets/c.toml');
const domCfg = path.resolve(ROOT, '../modules/dom.toml');
const bindingsDir = path.join(ROOT, 'bindings');

const selected = manifest.examples.filter(
  e => filters.length === 0 || filters.some(f => e.package === f || e.id === f || e.id.startsWith(`${f}/`)),
);
const neededPkgs = [...new Set(selected.map(e => e.package))];
const needsDom = neededPkgs.some(p => PACKAGES[p]?.needsDom);

fs.mkdirSync(bindingsDir, { recursive: true });
if (needsDom && !fs.existsSync(path.join(bindingsDir, 'dom/dom.h'))) {
  execFileSync(tsrsProto, ['lib.dom.d.ts', '--config', cfg, '--config', domCfg, '--emit', 'c', '--name', 'dom', '--out', bindingsDir], {
    cwd: ROOT,
    stdio: 'inherit',
  });
}

for (const pkg of neededPkgs) {
  const spec = PACKAGES[pkg];
  if (!spec) throw new Error(`Unknown package spec: ${pkg}`);
  if (!fs.existsSync(path.join(bindingsDir, spec.outDir))) {
    const args = ['--from', ROOT];
    for (const n of spec.npm) args.push('--package', n);
    args.push('--config', cfg, '--emit', 'c');
    if (spec.npm.length > 1) args.push('--name', spec.name);
    if (spec.needsDom) args.push('--ref', path.join(bindingsDir, 'dom'));
    args.push('--out', bindingsDir);
    execFileSync(tsrsProto, args, { cwd: ROOT, stdio: 'inherit' });
  }
}

for (const ex of selected) {
  const demoDir = path.join(ROOT, path.dirname(ex.path));
  console.log(`Building c ${ex.id}...`);
  execFileSync('clang', [...CLANG_WASM, '-I', bindingsDir, '-o', path.join(demoDir, 'app.wasm'), path.join(demoDir, 'main.c')], {
    stdio: 'inherit',
  });
}
