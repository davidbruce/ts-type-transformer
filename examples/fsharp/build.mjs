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
  pts: { npm: ['pts'], needsDom: false, outDir: 'Pts' },
  babylon_lite: { npm: ['@babylonjs/lite'], needsDom: true, outDir: 'Babylonjs.Lite' },
  katex: { npm: ['katex'], needsDom: true, outDir: 'Katex' },
  marked: { npm: ['marked'], needsDom: true, outDir: 'Marked' },
  date_fns: { npm: ['date-fns'], needsDom: false, outDir: 'DateFns' },
  node: { npm: ['fs', 'path', 'events'], needsDom: false, outDir: 'Node' },
  leaflet: { npm: ['leaflet'], needsDom: true, outDir: 'Leaflet' },
  pixi: { npm: ['pixi.js'], needsDom: true, outDir: 'PixiJs' },
  echarts: { npm: ['echarts'], needsDom: true, outDir: 'Echarts' },
  konva: { npm: ['konva'], needsDom: true, outDir: 'Konva' },
  three: { npm: ['three'], needsDom: true, outDir: 'Three' },
  rapier: { npm: ['@dimforge/rapier3d-compat'], needsDom: false, outDir: 'Dimforge.Rapier3dCompat' },
  tanstack_table: { npm: ['@tanstack/table-core'], needsDom: false, outDir: 'Tanstack.TableCore' },
  prosemirror: { npm: ['prosemirror-model', 'prosemirror-transform', 'prosemirror-state', 'prosemirror-view'], needsDom: true, outDir: 'Prosemirror' },
};

const tsrsProto =
  process.env.TSRS_PROTO ??
  ([path.resolve(ROOT, '../../target/debug/tsrs_proto'), path.resolve(ROOT, '../../target/release/tsrs_proto')]
    .filter(p => fs.existsSync(p))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)[0] || 'tsrs_proto');
const cfg = path.resolve(ROOT, '../targets/fsharp.toml');
const domCfg = path.resolve(ROOT, '../modules/dom.toml');
const bindingsDir = path.join(ROOT, 'bindings');
const fallbackFrom = path.resolve(ROOT, '../../target/libraries');
const fromDir = fs.existsSync(path.join(ROOT, 'node_modules/pts')) ? ROOT : fallbackFrom;

const selected = manifest.examples.filter(
  e => filters.length === 0 || filters.some(f => e.package === f || e.id === f || e.id.startsWith(`${f}/`)),
);
const neededPkgs = [...new Set(selected.map(e => e.package))];
const needsDom = neededPkgs.some(p => PACKAGES[p]?.needsDom);

fs.mkdirSync(bindingsDir, { recursive: true });
if (needsDom && !fs.existsSync(path.join(bindingsDir, 'Dom/Dom.fsproj'))) {
  execFileSync(tsrsProto, ['lib.dom.d.ts', '--config', cfg, '--config', domCfg, '--emit', 'fsharp', '--name', 'dom', '--out', bindingsDir], {
    cwd: ROOT,
    stdio: 'inherit',
  });
}

for (const pkg of neededPkgs) {
  const spec = PACKAGES[pkg];
  if (!spec) throw new Error(`Unknown package spec: ${pkg}`);
  const outFsproj = path.join(bindingsDir, spec.outDir, `${spec.outDir}.fsproj`);
  if (!fs.existsSync(outFsproj)) {
    const args = ['--from', fromDir];
    for (const n of spec.npm) args.push('--package', n);
    args.push('--config', cfg, '--emit', 'fsharp');
    if (spec.needsDom) args.push('--ref', path.join(bindingsDir, 'Dom'));
    args.push('--out', bindingsDir);
    execFileSync(tsrsProto, args, { cwd: ROOT, stdio: 'inherit' });
  }
}

const fableBin =
  [
    path.join(os.homedir(), '.dotnet/tools/fable'),
    path.join(os.homedir(), '.dotnet/tools/fable.exe'),
  ].find(p => fs.existsSync(p)) ?? 'fable';

for (const ex of selected) {
  const demoDir = path.join(ROOT, path.dirname(ex.path));
  const outPath = path.join(demoDir, 'js/Program.mjs');
  if (isUpToDate(ex, outPath)) continue;
  console.log(`Building fsharp ${ex.id}...`);
  const fableArgs = ['App.fsproj', '-o', 'js', '--extension', '.mjs'];
  if (process.env.FABLE_NO_CACHE === '1') fableArgs.push('--noCache');
  execFileSync(fableBin, fableArgs, {
    cwd: demoDir,
    stdio: 'inherit',
    env: { ...process.env, MSBUILDDISABLENODEREUSE: '1' },
  });
}
