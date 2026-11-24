import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';

const DEFAULT_PARALLEL_JOBS = Math.min(4, Math.max(1, os.cpus().length));

export const TARGETS = [
  {
    id: 'c',
    label: 'C (WASM)',
    shortLabel: 'C',
    artifactRel: 'app.wasm',
    usesGlue: true,
    defaultJobs: DEFAULT_PARALLEL_JOBS,
  },
  {
    id: 'cpp',
    label: 'C++ (WASM)',
    shortLabel: 'C++',
    artifactRel: 'app.wasm',
    usesGlue: true,
    defaultJobs: DEFAULT_PARALLEL_JOBS,
  },
  {
    id: 'zig',
    label: 'Zig (WASM)',
    shortLabel: 'Zig',
    artifactRel: 'app.wasm',
    usesGlue: true,
    defaultJobs: DEFAULT_PARALLEL_JOBS,
  },
  {
    id: 'rust',
    label: 'Rust (WASM)',
    shortLabel: 'Rust',
    artifactRel: 'app.wasm',
    usesGlue: true,
    defaultJobs: DEFAULT_PARALLEL_JOBS,
  },
  {
    id: 'gleam',
    label: 'Gleam (JS)',
    shortLabel: 'Gleam',
    artifactRel: 'build/dev/javascript/app/app.mjs',
    usesGlue: false,
    defaultJobs: DEFAULT_PARALLEL_JOBS,
  },
  {
    id: 'fsharp',
    label: 'F# (Fable)',
    shortLabel: 'F#',
    artifactRel: 'js/Program.mjs',
    usesGlue: false,
    defaultJobs: DEFAULT_PARALLEL_JOBS,
  },
];

export const TARGETS_BY_ID = Object.fromEntries(TARGETS.map(t => [t.id, t]));

export const PACKAGES_SNAKE = {
  pts: { npm: ['pts'], needsDom: false, outDir: 'pts', name: 'pts', cPrefix: 'pts' },
  babylon_lite: { npm: ['@babylonjs/lite'], needsDom: true, outDir: 'babylonjs_lite', name: 'babylonjs_lite', cPrefix: 'babylon' },
  katex: { npm: ['katex'], needsDom: true, outDir: 'katex', name: 'katex', cPrefix: 'katex' },
  marked: { npm: ['marked'], needsDom: true, outDir: 'marked', name: 'marked', cPrefix: 'marked' },
  date_fns: { npm: ['date-fns'], needsDom: true, outDir: 'date_fns', name: 'date_fns', cPrefix: 'df' },
  node: { npm: ['fs', 'path', 'events'], needsDom: false, outDir: 'node', name: 'node', cPrefix: 'node' },
  leaflet: { npm: ['leaflet'], needsDom: true, outDir: 'leaflet', name: 'leaflet', cPrefix: 'leaflet' },
  pixi: { npm: ['pixi.js'], needsDom: true, outDir: 'pixi_js', name: 'pixi_js', cPrefix: 'pixi' },
  echarts: { npm: ['echarts'], needsDom: true, outDir: 'echarts', name: 'echarts', cPrefix: 'echarts' },
  konva: { npm: ['konva'], needsDom: true, outDir: 'konva', name: 'konva', cPrefix: 'konva' },
  three: { npm: ['three'], needsDom: true, outDir: 'three', name: 'three', cPrefix: 'three' },
  rapier: { npm: ['@dimforge/rapier3d-compat'], needsDom: true, outDir: 'dimforge_rapier3d_compat', name: 'dimforge_rapier3d_compat', cPrefix: 'rapier' },
  tanstack_table: { npm: ['@tanstack/table-core'], needsDom: true, outDir: 'tanstack_table_core', name: 'tanstack_table_core', cPrefix: 'table' },
  prosemirror: {
    npm: ['prosemirror-model', 'prosemirror-transform', 'prosemirror-state', 'prosemirror-view'],
    needsDom: true,
    outDir: 'prosemirror',
    name: 'prosemirror',
    cPrefix: 'pm',
  },
};

export const PACKAGES_FSHARP = {
  pts: { npm: ['pts'], needsDom: false, outDir: 'Pts' },
  babylon_lite: { npm: ['@babylonjs/lite'], needsDom: true, outDir: 'Babylonjs.Lite' },
  katex: { npm: ['katex'], needsDom: true, outDir: 'Katex' },
  marked: { npm: ['marked'], needsDom: true, outDir: 'Marked' },
  date_fns: { npm: ['date-fns'], needsDom: true, outDir: 'DateFns' },
  node: { npm: ['fs', 'path', 'events'], needsDom: false, outDir: 'Node', name: 'node' },
  leaflet: { npm: ['leaflet'], needsDom: true, outDir: 'Leaflet' },
  pixi: { npm: ['pixi.js'], needsDom: true, outDir: 'Pixi.Js' },
  echarts: { npm: ['echarts'], needsDom: true, outDir: 'Echarts' },
  konva: { npm: ['konva'], needsDom: true, outDir: 'Konva' },
  three: { npm: ['three'], needsDom: true, outDir: 'Three' },
  rapier: { npm: ['@dimforge/rapier3d-compat'], needsDom: true, outDir: 'Dimforge.Rapier3dCompat' },
  tanstack_table: { npm: ['@tanstack/table-core'], needsDom: true, outDir: 'Tanstack.TableCore' },
  prosemirror: {
    npm: ['prosemirror-model', 'prosemirror-transform', 'prosemirror-state', 'prosemirror-view'],
    needsDom: true,
    outDir: 'Prosemirror',
    name: 'prosemirror',
  },
};

export function getPackages(targetId) {
  return targetId === 'fsharp' ? PACKAGES_FSHARP : PACKAGES_SNAKE;
}

export function readManifest(targetRoot) {
  return JSON.parse(fs.readFileSync(path.join(targetRoot, 'examples.json'), 'utf8'));
}

export function getArtifactPath(targetRoot, targetSpec, item) {
  return path.join(targetRoot, path.dirname(item.path), targetSpec.artifactRel);
}

export function getExamplePackages(item) {
  return [item.package, ...(item.extraPackages ?? [])];
}

export function isDemoCompiled(targetRoot, targetSpec, item) {
  if (!fs.existsSync(getArtifactPath(targetRoot, targetSpec, item))) {
    return false;
  }
  const pkgs = getPackages(targetSpec.id);
  const specs = getExamplePackages(item).map(p => pkgs[p]);
  if (specs.some(s => !s)) return false;
  const bindingsDir = path.join(targetRoot, 'bindings');
  const needsDom = specs.some(s => s.needsDom);
  if (targetSpec.usesGlue) {
    const demoDir = path.join(targetRoot, path.dirname(item.path));
    if (demoGlueFiles({ demoDir, specs, domDirName: 'dom' }).some(g => !fs.existsSync(g.file))) return false;
  } else if (targetSpec.id === 'gleam') {
    for (const s of specs) {
      if (!fs.existsSync(path.join(bindingsDir, s.outDir, 'gleam.toml'))) return false;
    }
    if (needsDom && !fs.existsSync(path.join(bindingsDir, 'dom', 'gleam.toml'))) return false;
  } else if (targetSpec.id === 'fsharp') {
    for (const s of specs) {
      if (!fs.existsSync(path.join(bindingsDir, s.outDir, `${s.outDir}.fsproj`))) return false;
    }
    if (needsDom && !fs.existsSync(path.join(bindingsDir, 'Dom', 'Dom.fsproj'))) return false;
  }
  return true;
}

export function getTargetStatus(examplesRoot, targetId) {
  const targetSpec = TARGETS_BY_ID[targetId];
  if (!targetSpec) {
    throw new Error(`Unknown target: ${targetId}`);
  }
  const targetRoot = path.join(examplesRoot, targetSpec.id);
  const manifest = readManifest(targetRoot);
  const items = {};
  let total = 0;
  let compiledCount = 0;
  let browserTotal = 0;
  let browserCompiled = 0;
  let nodeTotal = 0;
  let nodeCompiled = 0;

  for (const item of manifest.examples) {
    const compiled = isDemoCompiled(targetRoot, targetSpec, item);
    items[item.id] = compiled;
    total++;
    if (compiled) compiledCount++;
    if (item.kind === 'node') {
      nodeTotal++;
      if (compiled) nodeCompiled++;
    } else {
      browserTotal++;
      if (compiled) browserCompiled++;
    }
  }

  return {
    id: targetSpec.id,
    label: targetSpec.label,
    shortLabel: targetSpec.shortLabel,
    total,
    compiled: compiledCount,
    browserTotal,
    browserCompiled,
    nodeTotal,
    nodeCompiled,
    items,
  };
}

export function latestMtimeRecursive(dir, ext = null) {
  let latest = 0;
  if (!fs.existsSync(dir)) return latest;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('.') || entry.name === 'node_modules') continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      const sub = latestMtimeRecursive(full, ext);
      if (sub > latest) latest = sub;
    } else if (entry.isFile() && (!ext || entry.name.endsWith(ext))) {
      const mtime = fs.statSync(full).mtimeMs;
      if (mtime > latest) latest = mtime;
    }
  }
  return latest;
}

export function resolveTsrsProto(examplesRoot) {
  if (process.env.TSRS_PROTO) return process.env.TSRS_PROTO;
  const candidates = [
    path.resolve(examplesRoot, '../target/debug/tsrs_proto'),
    path.resolve(examplesRoot, '../target/release/tsrs_proto'),
  ]
    .filter(p => fs.existsSync(p))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
  return candidates[0] || 'tsrs_proto';
}

function parsePositiveInt(raw, flagName) {
  if (!/^\d+$/.test(raw)) {
    throw new Error(`Invalid value for ${flagName}: ${raw}`);
  }
  const n = Number.parseInt(raw, 10);
  if (!Number.isFinite(n) || n < 1) {
    throw new Error(`${flagName} must be a positive integer: ${raw}`);
  }
  return n;
}

export function parseBuildArgs(argv, defaultJobs) {
  let force = false;
  let browserOnly = false;
  let jobs = defaultJobs;
  const filters = [];

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--force') {
      force = true;
    } else if (arg === '--browser') {
      browserOnly = true;
    } else if (arg === '--jobs' || arg === '-j') {
      const next = argv[++i];
      if (!next) throw new Error(`Missing value after ${arg}`);
      jobs = parsePositiveInt(next, arg);
    } else if (arg.startsWith('--jobs=')) {
      jobs = parsePositiveInt(arg.slice('--jobs='.length), '--jobs');
    } else if (/^-j\d+$/.test(arg)) {
      jobs = parsePositiveInt(arg.slice(2), '-j');
    } else if (arg.startsWith('-')) {
      throw new Error(`Unknown option: ${arg}`);
    } else {
      filters.push(arg);
    }
  }

  return { force, browserOnly, jobs, filters };
}

export function selectExamples(manifest, filters, browserOnly) {
  return manifest.examples.filter(e => {
    if (browserOnly && e.kind === 'node') return false;
    if (filters.length === 0) return true;
    return filters.some(f => e.package === f || e.id === f || e.id.startsWith(`${f}/`));
  });
}

export async function runPool(items, concurrency, workerFn) {
  if (items.length === 0) return;
  const limit = Math.max(1, Math.min(concurrency, items.length));
  let nextIndex = 0;
  let firstError = null;

  const workers = Array.from({ length: limit }, async () => {
    while (!firstError) {
      const idx = nextIndex++;
      if (idx >= items.length) break;
      try {
        await workerFn(items[idx], idx);
      } catch (err) {
        if (!firstError) firstError = err;
        break;
      }
    }
  });

  await Promise.all(workers);
  if (firstError) throw firstError;
}

export function spawnAsync(cmd, args, { cwd, env, buffered = false } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      cwd,
      env: env ?? process.env,
      stdio: buffered ? ['ignore', 'pipe', 'pipe'] : 'inherit',
    });

    const outChunks = [];
    const errChunks = [];
    if (buffered) {
      child.stdout?.on('data', chunk => outChunks.push(chunk));
      child.stderr?.on('data', chunk => errChunks.push(chunk));
    }

    const flush = () => {
      if (!buffered) return;
      const out = Buffer.concat(outChunks);
      const err = Buffer.concat(errChunks);
      if (out.length > 0) process.stdout.write(out);
      if (err.length > 0) process.stderr.write(err);
    };

    child.on('error', err => {
      flush();
      reject(err);
    });
    child.on('exit', (code, signal) => {
      flush();
      if (code === 0) {
        resolve();
      } else {
        const why = signal ? `signal ${signal}` : `exit code ${code}`;
        reject(new Error(`${cmd} ${args.join(' ')} failed (${why})`));
      }
    });
  });
}

export const VITE_OPTIMIZE_EXCLUDE = ['@babylonjs/lite', 'three', 'echarts'];

export const VITE_WATCH_IGNORED = [
  '**/node_modules/**',
  '**/bindings/**',
  '**/bin/**',
  '**/obj/**',
  '**/js/**',
  '**/_all/**',
  '**/build/**',
  '**/zig-out/**',
  '**/.zig-cache/**',
  '**/*.wasm',
  '**/manifest.toml',
];

export function createFallbackLibrariesPlugin(fallbackNodeModules, fallbackPkgJson) {
  return {
    name: 'fallback-libraries-node-modules',
    async resolveId(id, importer) {
      if (
        !fs.existsSync(fallbackNodeModules) ||
        id.startsWith('.') ||
        id.startsWith('/') ||
        id.startsWith('\0') ||
        id.startsWith('http:') ||
        id.startsWith('https:') ||
        id.startsWith('data:') ||
        id.startsWith('node:')
      ) {
        return null;
      }
      const resolved = await this.resolve(id, importer, { skipSelf: true });
      if (resolved) return resolved;
      return this.resolve(id, fallbackPkgJson, { skipSelf: true });
    },
  };
}

export function buildErrorHtml(err) {
  const msg = String(err?.message ?? err)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  return `<!doctype html><html><body style="font-family:monospace;padding:20px;background:#fff;color:#111"><h1>Build failed</h1><pre>${msg}</pre></body></html>`;
}

export function normalizeHtmlPathname(rawUrl) {
  const pathname = (rawUrl ?? '').split('?')[0].split('#')[0];
  return pathname.endsWith('/') ? `${pathname}index.html` : pathname;
}

export function sendJson(res, statusCode, payload) {
  res.statusCode = statusCode;
  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  res.setHeader('Cache-Control', 'no-store');
  res.end(JSON.stringify(payload));
}

export function createQueuedDemoBuilder(examplesRoot) {
  const inFlight = new Map();
  const buildChains = new Map();

  function buildDemoOnce(targetId, demoId) {
    const key = `${targetId}:${demoId}`;
    let promise = inFlight.get(key);
    if (promise) return promise;

    const targetRoot = path.join(examplesRoot, targetId);
    const prevChain = buildChains.get(targetId) ?? Promise.resolve();
    promise = prevChain
      .catch(() => {})
      .then(() =>
        spawnAsync(process.execPath, [path.join(targetRoot, 'build.mjs'), demoId], {
          cwd: targetRoot,
          env: { ...process.env, MSBUILDDISABLENODEREUSE: '1' },
          buffered: false,
        }),
      )
      .finally(() => {
        inFlight.delete(key);
      });

    buildChains.set(targetId, promise);
    inFlight.set(key, promise);
    return promise;
  }

  return {
    async ensureBuiltAndServe(server, res, next, targetId, item) {
      const targetSpec = TARGETS_BY_ID[targetId];
      const targetRoot = path.join(examplesRoot, targetId);
      if (!isDemoCompiled(targetRoot, targetSpec, item)) {
        try {
          await buildDemoOnce(targetId, item.id);
          const artifact = getArtifactPath(targetRoot, targetSpec, item);
          server.watcher?.emit('addDir', path.dirname(artifact));
          server.watcher?.emit('add', artifact);
        } catch (err) {
          res.statusCode = 500;
          res.setHeader('Content-Type', 'text/html; charset=utf-8');
          res.end(buildErrorHtml(err));
          return;
        }
      }
      next();
    },
  };
}

export function createTargetViteConfig(targetRoot, targetId) {
  const TARGET = TARGETS_BY_ID[targetId];
  if (!TARGET) throw new Error(`Unknown target: ${targetId}`);
  const examplesRoot = path.resolve(targetRoot, '..');
  const fallbackPkgJson = path.resolve(targetRoot, '../../target/libraries/package.json');
  const fallbackNodeModules = path.resolve(targetRoot, '../../target/libraries/node_modules');
  const builder = createQueuedDemoBuilder(examplesRoot);

  return {
    root: targetRoot,
    appType: 'mpa',
    optimizeDeps: {
      exclude: VITE_OPTIMIZE_EXCLUDE,
      entries: [],
    },
    server: {
      fs: {
        allow: [targetRoot, path.resolve(targetRoot, '../..')],
      },
      watch: {
        ignored: VITE_WATCH_IGNORED,
      },
    },
    plugins: [
      createFallbackLibrariesPlugin(fallbackNodeModules, fallbackPkgJson),
      {
        name: 'auto-build-examples',
        resolveId(id) {
          if (id === '/code_viewer.mjs') {
            return path.join(examplesRoot, 'code_viewer.mjs');
          }
          if (id === '/target_gallery.mjs') {
            return path.join(examplesRoot, 'target_gallery.mjs');
          }
          return null;
        },
        configureServer(server) {
          server.middlewares.use(async (req, res, next) => {
            const url = new URL(req.url ?? '/', 'http://localhost');
            if (req.method === 'GET' && url.pathname === '/__api/status') {
              return sendJson(res, 200, { target: getTargetStatus(examplesRoot, TARGET.id) });
            }
            if (req.method === 'GET' && (url.pathname === '/code_viewer.mjs' || url.pathname === '/target_gallery.mjs')) {
              const file = url.pathname.slice(1);
              res.statusCode = 200;
              res.setHeader('Content-Type', 'text/javascript; charset=utf-8');
              res.end(fs.readFileSync(path.join(examplesRoot, file), 'utf8'));
              return;
            }
            const pathname = normalizeHtmlPathname(req.url);
            const manifest = readManifest(targetRoot);
            const item = manifest.examples.find(
              e => e.kind !== 'node' && `/${e.path.replace(/^\.\//, '')}` === pathname,
            );
            if (!item) return next();
            await builder.ensureBuiltAndServe(server, res, next, TARGET.id, item);
          });
        },
      },
    ],
  };
}

// The generator's arguments for the DOM and for a package, but `--out`.
function domArgs({ cfg, domCfg, emit }) {
  return ['lib.dom.d.ts', '--config', cfg, '--config', domCfg, '--emit', emit, '--name', 'dom'];
}

function packageArgs({ spec, fromDir, cfg, emit, domDir }) {
  const args = ['--from', fromDir];
  for (const n of spec.npm) args.push('--package', n);
  args.push('--config', cfg, '--emit', emit);
  if (emit === 'c' && spec.cPrefix) args.push('--prefix', spec.cPrefix);
  if (spec.npm.length > 1) args.push('--name', spec.name);
  if (spec.needsDom) args.push('--ref', domDir);
  return args;
}

// Where a wasm demo's glue is: only what its module imports, beside it.
function demoGlueFiles({ demoDir, specs, domDirName }) {
  const files = [];
  if (specs.some(s => s.needsDom)) files.push({ dom: true, file: path.join(demoDir, 'glue', `${domDirName}.mjs`) });
  for (const spec of new Map(specs.map(s => [s.outDir, s])).values()) {
    files.push({ spec, file: path.join(demoDir, 'glue', `${spec.outDir}.mjs`) });
  }
  return files;
}

async function trimDemoGlue({ tsrsProto, targetRoot, cfg, domCfg, emit, fromDir, bindingsDir, domDirName, demoDir, outPath, specs, buffered }) {
  const domDir = path.join(bindingsDir, domDirName);
  for (const { dom, spec, file } of demoGlueFiles({ demoDir, specs, domDirName })) {
    const args = dom ? domArgs({ cfg, domCfg, emit }) : packageArgs({ spec, fromDir, cfg, emit, domDir });
    await spawnAsync(tsrsProto, [...args, '--out', bindingsDir, '--glue-for', outPath, '--glue-out', file], { cwd: targetRoot, buffered });
  }
}

export async function ensureTargetBindings({
  targetRoot,
  emit,
  tsrsProto,
  cfg,
  domCfg,
  fromDir,
  bindingsDir,
  packages,
  neededPkgs,
  needsDom,
  force,
  domDirName = 'dom',
  domSentinel,
  pkgSentinel,
  onRegenerate,
}) {
  fs.mkdirSync(bindingsDir, { recursive: true });
  const domDir = path.join(bindingsDir, domDirName);
  const domSentinelPath = path.join(domDir, domSentinel);
  const domMetaPath = path.join(domDir, 'tsrs-meta.toml');

  if (needsDom && (force || !fs.existsSync(domSentinelPath) || !fs.existsSync(domMetaPath))) {
    if (force) {
      fs.rmSync(domDir, { recursive: true, force: true });
    }
    onRegenerate?.(domDirName);
    await spawnAsync(
      tsrsProto,
      [...domArgs({ cfg, domCfg, emit }), '--out', bindingsDir],
      { cwd: targetRoot },
    );
  }

  for (const pkg of neededPkgs) {
    const spec = packages[pkg];
    if (!spec) throw new Error(`Unknown package spec: ${pkg}`);
    const pkgDir = path.join(bindingsDir, spec.outDir);
    const sentinelRel = typeof pkgSentinel === 'function' ? pkgSentinel(spec) : pkgSentinel;
    const pkgSentinelPath = path.join(pkgDir, sentinelRel);
    const pkgMetaPath = path.join(pkgDir, 'tsrs-meta.toml');
    if (force || !fs.existsSync(pkgSentinelPath) || !fs.existsSync(pkgMetaPath)) {
      if (force) {
        fs.rmSync(pkgDir, { recursive: true, force: true });
      }
      onRegenerate?.(spec.outDir);
      await spawnAsync(tsrsProto, [...packageArgs({ spec, fromDir, cfg, emit, domDir }), '--out', bindingsDir], { cwd: targetRoot });
    }
  }
}

export function isDemoOutputCurrent({
  root,
  bindingsDir,
  targetSpec,
  packages,
  ex,
  demoDir,
  outPath,
  force,
  extraFiles = [],
  srcDir = null,
  bindingExt = null,
  domDirName = 'dom',
}) {
  if (force || !isDemoCompiled(root, targetSpec, ex)) return false;
  const outMtime = fs.statSync(outPath).mtimeMs;
  const srcPath = path.resolve(root, ex.source);
  if (!fs.existsSync(srcPath) || fs.statSync(srcPath).mtimeMs > outMtime) return false;
  if (srcDir && latestMtimeRecursive(srcDir) > outMtime) return false;
  for (const rel of extraFiles) {
    const p = path.join(demoDir, rel);
    if (fs.existsSync(p) && fs.statSync(p).mtimeMs > outMtime) return false;
  }
  const specs = getExamplePackages(ex).map(p => packages[p]);
  for (const spec of specs) {
    if (!spec) return false;
    if (latestMtimeRecursive(path.join(bindingsDir, spec.outDir), bindingExt) > outMtime) return false;
  }
  if (specs.some(s => s.needsDom) && latestMtimeRecursive(path.join(bindingsDir, domDirName), bindingExt) > outMtime) {
    return false;
  }
  if (targetSpec.usesGlue && demoGlueFiles({ demoDir, specs, domDirName }).some(g => !fs.existsSync(g.file) || fs.statSync(g.file).mtimeMs < outMtime)) {
    return false;
  }
  return true;
}

export async function runTargetBuild({
  targetId,
  root,
  emit = targetId,
  cfgName = `${targetId}.toml`,
  domDirName = 'dom',
  domSentinel,
  pkgSentinel,
  extraFiles = [],
  srcSubdir = null,
  bindingExt = null,
  onRegenerate,
  beforeDemos,
  buildDemo,
}) {
  const targetSpec = TARGETS_BY_ID[targetId];
  const packages = getPackages(targetId);
  const manifest = readManifest(root);
  const { force, browserOnly, jobs, filters } = parseBuildArgs(process.argv.slice(2), targetSpec.defaultJobs);
  const selected = selectExamples(manifest, filters, browserOnly);

  if (selected.length === 0) {
    console.log(`[${targetId}] Done: 0 built, 0 up-to-date (0 total)`);
    return;
  }

  const neededPkgs = [...new Set(selected.flatMap(getExamplePackages))];
  const needsDom = neededPkgs.some(p => packages[p]?.needsDom);
  const tsrsProto = resolveTsrsProto(path.resolve(root, '..'));
  const cfg = path.resolve(root, `../targets/${cfgName}`);
  const domCfg = path.resolve(root, '../modules/dom.toml');
  const bindingsDir = path.join(root, 'bindings');
  const sharedExamplesDir = path.resolve(root, '..');
  const fallbackFrom = path.resolve(root, '../../target/libraries');
  const fromDir = fs.existsSync(path.join(root, 'node_modules/pts'))
    ? root
    : fs.existsSync(path.join(sharedExamplesDir, 'node_modules/pts'))
      ? sharedExamplesDir
      : fallbackFrom;

  await ensureTargetBindings({
    targetRoot: root,
    emit,
    tsrsProto,
    cfg,
    domCfg,
    fromDir,
    bindingsDir,
    packages,
    neededPkgs,
    needsDom,
    force,
    domDirName,
    domSentinel,
    pkgSentinel,
    onRegenerate: dirName => onRegenerate?.(dirName, bindingsDir),
  });

  const toBuild = [];
  for (const ex of selected) {
    const demoDir = path.join(root, path.dirname(ex.path));
    const outPath = getArtifactPath(root, targetSpec, ex);
    const upToDate = isDemoOutputCurrent({
      root,
      bindingsDir,
      targetSpec,
      packages,
      ex,
      demoDir,
      outPath,
      force,
      extraFiles,
      srcDir: srcSubdir ? path.join(demoDir, srcSubdir) : null,
      bindingExt,
      domDirName,
    });
    if (!upToDate) {
      const specs = getExamplePackages(ex).map(p => packages[p]);
      toBuild.push({ ex, demoDir, outPath, spec: specs[0], specs });
    }
  }

  // `beforeDemos` returns true when it built every demo itself.
  let builtAll = false;
  if (toBuild.length > 0 && beforeDemos) {
    const buildPkgs = [...new Set(toBuild.flatMap(t => getExamplePackages(t.ex)))];
    const buildDom = buildPkgs.some(p => packages[p]?.needsDom);
    builtAll = (await beforeDemos({ toBuild, buildPkgs, buildDom, packages, bindingsDir, jobs, force })) === true;
  }

  const buffered = jobs > 1;
  await runPool(builtAll ? [] : toBuild, jobs, async ({ ex, demoDir, outPath, spec, specs }) => {
    console.log(`Building ${targetId} ${ex.id}...`);
    await buildDemo({ ex, demoDir, outPath, spec, specs, bindingsDir, jobs, buffered });
    if (targetSpec.usesGlue) {
      await trimDemoGlue({ tsrsProto, targetRoot: root, cfg, domCfg, emit, fromDir, bindingsDir, domDirName, demoDir, outPath, specs, buffered });
    }
  });

  const built = toBuild.length;
  const skipped = selected.length - built;
  console.log(`[${targetId}] Done: ${built} built, ${skipped} up-to-date (${selected.length} total)`);
}
