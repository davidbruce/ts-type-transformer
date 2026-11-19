import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, 'examples.json'), 'utf8'));
const filters = process.argv.slice(2);

const PACKAGES = {
  pts: { npm: ['pts'], needsDom: false, outDir: 'Pts' },
  babylon_lite: { npm: ['@babylonjs/lite'], needsDom: true, outDir: 'Babylonjs.Lite' },
  katex: { npm: ['katex'], needsDom: true, outDir: 'Katex' },
  marked: { npm: ['marked'], needsDom: false, outDir: 'Marked' },
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
  ([path.resolve(ROOT, '../../target/release/tsrs_proto'), path.resolve(ROOT, '../../target/debug/tsrs_proto')].find(p =>
    fs.existsSync(p),
  ) || 'tsrs_proto');
const cfg = path.resolve(ROOT, '../targets/fsharp.toml');
const domCfg = path.resolve(ROOT, '../modules/dom.toml');
const bindingsDir = path.join(ROOT, 'bindings');

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
    const args = ['--from', ROOT];
    for (const n of spec.npm) args.push('--package', n);
    args.push('--config', cfg, '--emit', 'fsharp');
    if (spec.needsDom) args.push('--ref', path.join(bindingsDir, 'Dom'));
    args.push('--out', bindingsDir);
    execFileSync(tsrsProto, args, { cwd: ROOT, stdio: 'inherit' });
  }
}

for (const ex of selected) {
  const demoDir = path.join(ROOT, path.dirname(ex.path));
  console.log(`Building fsharp ${ex.id}...`);
  execFileSync('fable', ['App.fsproj', '-o', 'js', '--extension', '.mjs', '--noCache'], {
    cwd: demoDir,
    stdio: 'inherit',
  });
}
