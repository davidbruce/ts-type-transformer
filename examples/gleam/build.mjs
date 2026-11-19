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

const STDLIB_CHECKSUM = 'CEE5B6C076A85B45F60C585F4316C63EC8B7127C119D5738C3958A9C4D50404E';
const tsrsProto =
  process.env.TSRS_PROTO ??
  ([path.resolve(ROOT, '../../target/debug/tsrs_proto'), path.resolve(ROOT, '../../target/release/tsrs_proto')]
    .filter(p => fs.existsSync(p))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs)[0] || 'tsrs_proto');
const cfg = path.resolve(ROOT, '../targets/gleam.toml');
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
if (needsDom && !fs.existsSync(path.join(bindingsDir, 'dom/gleam.toml'))) {
  execFileSync(tsrsProto, ['lib.dom.d.ts', '--config', cfg, '--config', domCfg, '--emit', 'gleam', '--name', 'dom', '--out', bindingsDir], {
    cwd: ROOT,
    stdio: 'inherit',
  });
}

for (const pkg of neededPkgs) {
  const spec = PACKAGES[pkg];
  if (!spec) throw new Error(`Unknown package spec: ${pkg}`);
  if (!fs.existsSync(path.join(bindingsDir, spec.outDir))) {
    const args = ['--from', fromDir];
    for (const n of spec.npm) args.push('--package', n);
    args.push('--config', cfg, '--emit', 'gleam');
    if (spec.npm.length > 1) args.push('--name', spec.name);
    if (spec.needsDom) args.push('--ref', path.join(bindingsDir, 'dom'));
    args.push('--out', bindingsDir);
    execFileSync(tsrsProto, args, { cwd: ROOT, stdio: 'inherit' });
  }
}

for (const ex of selected) {
  const spec = PACKAGES[ex.package];
  const demoDir = path.join(ROOT, path.dirname(ex.path));
  const outPath = path.join(demoDir, 'build/dev/javascript/app/app.mjs');
  if (isUpToDate(ex, outPath)) continue;
  const pkgEntries = [];
  const reqEntries = [`gleam_stdlib = { version = ">= 1.0.0 and < 2.0.0" }`];
  if (spec.needsDom) {
    pkgEntries.push(
      `  { name = "${spec.outDir}", version = "0.1.0", build_tools = ["gleam"], requirements = ["dom", "gleam_stdlib"], source = "local", path = "../../bindings/${spec.outDir}" },`,
      `  { name = "dom", version = "0.1.0", build_tools = ["gleam"], requirements = ["gleam_stdlib"], source = "local", path = "../../bindings/dom" },`,
    );
    reqEntries.unshift(`${spec.outDir} = { path = "../../bindings/${spec.outDir}" }`, `dom = { path = "../../bindings/dom" }`);
  } else {
    pkgEntries.push(
      `  { name = "${spec.outDir}", version = "0.1.0", build_tools = ["gleam"], requirements = ["gleam_stdlib"], source = "local", path = "../../bindings/${spec.outDir}" },`,
    );
    reqEntries.unshift(`${spec.outDir} = { path = "../../bindings/${spec.outDir}" }`);
  }
  pkgEntries.push(
    `  { name = "gleam_stdlib", version = "1.0.5", build_tools = ["gleam"], requirements = [], otp_app = "gleam_stdlib", source = "hex", outer_checksum = "${STDLIB_CHECKSUM}" },`,
  );
  fs.writeFileSync(
    path.join(demoDir, 'manifest.toml'),
    `packages = [\n${pkgEntries.join('\n')}\n]\n\n[requirements]\n${reqEntries.join('\n')}\n`,
  );
  console.log(`Building gleam ${ex.id}...`);
  execFileSync('gleam', ['build', '--warnings-as-errors'], { cwd: demoDir, stdio: 'inherit' });
}
