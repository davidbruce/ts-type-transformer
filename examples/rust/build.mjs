import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { latestMtimeRecursive, runPool, runTargetBuild, spawnAsync } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const RUSTC_WASM = ['--edition', '2024', '--target', 'wasm32-unknown-unknown', '-O', '-D', 'warnings'];

const rlibCache = new Map();

async function ensureRlib(crateName, bindingsDir, rlibsDir, externs, force, buffered) {
  const out = path.join(rlibsDir, `lib${crateName}.rlib`);
  const srcDir = path.join(bindingsDir, crateName);
  const outMtime = fs.existsSync(out) ? fs.statSync(out).mtimeMs : 0;
  const depNewer = externs.some(([, depPath]) => fs.statSync(depPath).mtimeMs > outMtime);
  const srcNewer = latestMtimeRecursive(srcDir) > outMtime;

  if (force || outMtime === 0 || depNewer || srcNewer) {
    const args = [...RUSTC_WASM, '--crate-type', 'rlib', '--crate-name', crateName, '-L', rlibsDir];
    for (const [n, p] of externs) args.push('--extern', `${n}=${p}`);
    args.push('-o', out, path.join(srcDir, 'src/lib.rs'));
    await spawnAsync('rustc', args, { buffered });
  }

  rlibCache.set(crateName, out);
  return out;
}

await runTargetBuild({
  targetId: 'rust',
  root: ROOT,
  domSentinel: 'src/lib.rs',
  pkgSentinel: 'glue.mjs',
  extraFiles: ['Cargo.toml'],
  srcSubdir: 'src',
  onRegenerate: (dirName, bindingsDir) => {
    fs.rmSync(path.join(bindingsDir, '.rlibs', `lib${dirName}.rlib`), { force: true });
  },
  beforeDemos: async ({ buildPkgs, buildDom, packages, bindingsDir, jobs, force }) => {
    const rlibsDir = path.join(bindingsDir, '.rlibs');
    fs.mkdirSync(rlibsDir, { recursive: true });
    // Every crate shares the runtime's crate, written beside them.
    const runtimeRlib = await ensureRlib('ts_runtime', bindingsDir, rlibsDir, [], force, false);
    const rt = ['ts_runtime', runtimeRlib];
    let domRlib = null;
    if (buildDom) {
      domRlib = await ensureRlib('dom', bindingsDir, rlibsDir, [rt], force, false);
    }
    const specs = [...new Map(buildPkgs.map(p => [packages[p].outDir, packages[p]])).values()];
    await runPool(specs, jobs, spec =>
      ensureRlib(
        spec.outDir,
        bindingsDir,
        rlibsDir,
        spec.needsDom ? [rt, ['dom', domRlib]] : [rt],
        force,
        jobs > 1,
      ),
    );
  },
  buildDemo: ({ demoDir, outPath, specs, bindingsDir, buffered }) => {
    const rlibsDir = path.join(bindingsDir, '.rlibs');
    const externs = [];
    const seen = new Set();
    if (specs.some(s => s.needsDom)) externs.push(['dom', rlibCache.get('dom')]);
    for (const s of specs) {
      if (seen.has(s.outDir)) continue;
      seen.add(s.outDir);
      externs.push([s.outDir, rlibCache.get(s.outDir)]);
    }
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
    return spawnAsync('rustc', args, { buffered });
  },
});
