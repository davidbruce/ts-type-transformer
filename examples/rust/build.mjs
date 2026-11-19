import fs from 'node:fs';
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
  date_fns: { npm: ['date-fns'], needsDom: true, outDir: 'date_fns', name: 'date_fns' },
  node: { npm: ['fs', 'path', 'events'], needsDom: false, outDir: 'node', name: 'node' },
  leaflet: { npm: ['leaflet'], needsDom: true, outDir: 'leaflet', name: 'leaflet' },
  pixi: { npm: ['pixi.js'], needsDom: true, outDir: 'pixi_js', name: 'pixi_js' },
  echarts: { npm: ['echarts'], needsDom: true, outDir: 'echarts', name: 'echarts' },
  konva: { npm: ['konva'], needsDom: true, outDir: 'konva', name: 'konva' },
  three: { npm: ['three'], needsDom: true, outDir: 'three', name: 'three' },
  rapier: { npm: ['@dimforge/rapier3d-compat'], needsDom: false, outDir: 'dimforge_rapier3d_compat', name: 'dimforge_rapier3d_compat' },
  tanstack_table: { npm: ['@tanstack/table-core'], needsDom: true, outDir: 'tanstack_table_core', name: 'tanstack_table_core' },
  prosemirror: { npm: ['prosemirror-model', 'prosemirror-transform', 'prosemirror-state', 'prosemirror-view'], needsDom: true, outDir: 'prosemirror', name: 'prosemirror' },
};

const RUSTC_WASM = ['--edition', '2024', '--target', 'wasm32-unknown-unknown', '-O', '-D', 'warnings'];

const tsrsProto =
  process.env.TSRS_PROTO ??
  ([path.resolve(ROOT, '../../target/debug/tsrs_proto'), path.resolve(ROOT, '../../target/release/tsrs_proto')]
    .filter(p => fs.existsSync(p))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)[0] || 'tsrs_proto');
const cfg = path.resolve(ROOT, '../targets/rust.toml');
const domCfg = path.resolve(ROOT, '../modules/dom.toml');
const bindingsDir = path.join(ROOT, 'bindings');
const rlibsDir = path.join(bindingsDir, '.rlibs');
const fallbackFrom = path.resolve(ROOT, '../../target/libraries');
const fromDir = fs.existsSync(path.join(ROOT, 'node_modules/pts')) ? ROOT : fallbackFrom;

const selected = manifest.examples.filter(
  e => filters.length === 0 || filters.some(f => e.package === f || e.id === f || e.id.startsWith(`${f}/`)),
);
const neededPkgs = [...new Set(selected.map(e => e.package))];
const needsDom = neededPkgs.some(p => PACKAGES[p]?.needsDom);

fs.mkdirSync(rlibsDir, { recursive: true });
if (needsDom && !fs.existsSync(path.join(bindingsDir, 'dom/src/lib.rs'))) {
  execFileSync(tsrsProto, ['lib.dom.d.ts', '--config', cfg, '--config', domCfg, '--emit', 'rust', '--name', 'dom', '--out', bindingsDir], {
    cwd: ROOT,
    stdio: 'inherit',
  });
}

function ensureRlib(crateName, externs = []) {
  const out = path.join(rlibsDir, `lib${crateName}.rlib`);
  if (!fs.existsSync(out)) {
    const args = [...RUSTC_WASM, '--crate-type', 'rlib', '--crate-name', crateName, '-L', rlibsDir];
    for (const [n, p] of externs) args.push('--extern', `${n}=${p}`);
    args.push('-o', out, path.join(bindingsDir, crateName, 'src/lib.rs'));
    execFileSync('rustc', args, { stdio: 'inherit' });
  }
  return out;
}

for (const pkg of neededPkgs) {
  const spec = PACKAGES[pkg];
  if (!spec) throw new Error(`Unknown package spec: ${pkg}`);
  if (!fs.existsSync(path.join(bindingsDir, spec.outDir))) {
    const args = ['--from', fromDir];
    for (const n of spec.npm) args.push('--package', n);
    args.push('--config', cfg, '--emit', 'rust');
    if (spec.npm.length > 1) args.push('--name', spec.name);
    if (spec.needsDom) args.push('--ref', path.join(bindingsDir, 'dom'));
    args.push('--out', bindingsDir);
    execFileSync(tsrsProto, args, { cwd: ROOT, stdio: 'inherit' });
  }
}

let domRlib = null;
for (const ex of selected) {
  const demoDir = path.join(ROOT, path.dirname(ex.path));
  const outPath = path.join(demoDir, 'app.wasm');
  if (isUpToDate(ex, outPath)) continue;
  const spec = PACKAGES[ex.package];
  const externs = [];
  if (spec.needsDom) {
    if (domRlib === null) domRlib = ensureRlib('dom');
    externs.push(['dom', domRlib]);
    externs.push([spec.outDir, ensureRlib(spec.outDir, [['dom', domRlib]])]);
  } else {
    externs.push([spec.outDir, ensureRlib(spec.outDir)]);
  }
  console.log(`Building rust ${ex.id}...`);
  const args = [
    ...RUSTC_WASM,
    '--crate-type',
    'cdylib',
    '-C',
    'link-arg=--export-table',
    '-C',
    'strip=debuginfo',
    '-L',
    rlibsDir,
  ];
  for (const [n, p] of externs) args.push('--extern', `${n}=${p}`);
  args.push('-o', outPath, path.join(demoDir, 'src/lib.rs'));
  execFileSync('rustc', args, { stdio: 'inherit' });
}
