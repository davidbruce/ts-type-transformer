import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  TARGETS,
  TARGETS_BY_ID,
  getTargetStatus,
  resolveTsrsProto,
  spawnAsync,
} from './targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));

function printHelp() {
  console.log(`Usage: node build.mjs [options] [target | target:filter ...]

Options:
  --browser          Only compile browser examples (skip kind === "node")
  --force            Regenerate bindings and force rebuild all selected demos
  --jobs <N>, -j <N> Max parallel compilations per target (fsharp defaults to 1)
  --list, --status   Print pre-compiled status table without building
  -h, --help         Show this help message

Targets:
  ${TARGETS.map(t => t.id).join(', ')}

Examples:
  node build.mjs                         Pre-build all 6 targets
  node build.mjs --browser               Pre-build all browser demos (skip node CLI)
  node build.mjs c rust                  Pre-build C and Rust demos
  node build.mjs c:pts fsharp:katex/inline_vs_display
  node build.mjs --list                  Show compiled/total counts per language
`);
}

function findExecutableInPath(name) {
  const dirs = (process.env.PATH ?? '').split(path.delimiter).filter(Boolean);
  for (const dir of dirs) {
    const candidate = path.join(dir, name);
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function assertTsrsProtoExists() {
  const bin = resolveTsrsProto(ROOT);
  const exists =
    bin.includes('/') || bin.includes('\\') ? fs.existsSync(bin) : Boolean(findExecutableInPath(bin));
  if (!exists) {
    throw new Error(
      `tsrs_proto binary not found (${bin}). Build it with \`cargo build\` or set TSRS_PROTO.`,
    );
  }
}

function parseTopArgs(argv) {
  let force = false;
  let browserOnly = false;
  let jobsArg = null;
  let listOnly = false;
  let help = false;
  const targetSelections = new Map();

  const ensureTarget = targetId => {
    if (!targetSelections.has(targetId)) {
      targetSelections.set(targetId, { all: false, filters: [] });
    }
    return targetSelections.get(targetId);
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '-h' || arg === '--help') {
      help = true;
    } else if (arg === '--list' || arg === '--status') {
      listOnly = true;
    } else if (arg === '--force') {
      force = true;
    } else if (arg === '--browser') {
      browserOnly = true;
    } else if (arg === '--jobs' || arg === '-j') {
      const next = argv[++i];
      if (!next || !/^\d+$/.test(next) || Number.parseInt(next, 10) < 1) {
        throw new Error(`Invalid value for ${arg}: ${next ?? '<missing>'}`);
      }
      jobsArg = next;
    } else if (arg.startsWith('--jobs=')) {
      const val = arg.slice('--jobs='.length);
      if (!/^\d+$/.test(val) || Number.parseInt(val, 10) < 1) {
        throw new Error(`Invalid value for --jobs: ${val}`);
      }
      jobsArg = val;
    } else if (/^-j\d+$/.test(arg)) {
      const val = arg.slice(2);
      if (Number.parseInt(val, 10) < 1) {
        throw new Error(`Invalid value for -j: ${val}`);
      }
      jobsArg = val;
    } else if (arg.startsWith('-')) {
      throw new Error(`Unknown option: ${arg}`);
    } else if (TARGETS_BY_ID[arg]) {
      ensureTarget(arg).all = true;
    } else if (arg.includes(':')) {
      const idx = arg.indexOf(':');
      const targetId = arg.slice(0, idx);
      const filter = arg.slice(idx + 1);
      if (!TARGETS_BY_ID[targetId]) {
        throw new Error(
          `Unknown target "${targetId}" in "${arg}". Valid targets: ${TARGETS.map(t => t.id).join(', ')}`,
        );
      }
      if (!filter) {
        throw new Error(`Missing filter after "${targetId}:" in "${arg}"`);
      }
      ensureTarget(targetId).filters.push(filter);
    } else {
      throw new Error(
        `Unknown argument "${arg}". Specify a target (${TARGETS.map(t => t.id).join(', ')}) or target:filter.`,
      );
    }
  }

  if (targetSelections.size === 0) {
    for (const t of TARGETS) {
      targetSelections.set(t.id, { all: true, filters: [] });
    }
  }

  return { help, listOnly, force, browserOnly, jobsArg, targetSelections };
}

function padRight(s, width) {
  return String(s).padEnd(width, ' ');
}

function padLeft(s, width) {
  return String(s).padStart(width, ' ');
}

function printStatusTable(targetIds) {
  const rows = targetIds.map(id => getTargetStatus(ROOT, id));
  const totals = rows.reduce(
    (acc, r) => ({
      browserCompiled: acc.browserCompiled + r.browserCompiled,
      browserTotal: acc.browserTotal + r.browserTotal,
      nodeCompiled: acc.nodeCompiled + r.nodeCompiled,
      nodeTotal: acc.nodeTotal + r.nodeTotal,
      compiled: acc.compiled + r.compiled,
      total: acc.total + r.total,
    }),
    { browserCompiled: 0, browserTotal: 0, nodeCompiled: 0, nodeTotal: 0, compiled: 0, total: 0 },
  );

  console.log('');
  console.log(
    `${padRight('Target', 16)} ${padLeft('Browser', 10)} ${padLeft('Node', 8)} ${padLeft('Total', 10)}`,
  );
  console.log('-'.repeat(47));
  for (const r of rows) {
    console.log(
      `${padRight(`${r.id} (${r.shortLabel})`, 16)} ${padLeft(`${r.browserCompiled}/${r.browserTotal}`, 10)} ${padLeft(`${r.nodeCompiled}/${r.nodeTotal}`, 8)} ${padLeft(`${r.compiled}/${r.total}`, 10)}`,
    );
  }
  if (rows.length > 1) {
    console.log('-'.repeat(47));
    console.log(
      `${padRight('All', 16)} ${padLeft(`${totals.browserCompiled}/${totals.browserTotal}`, 10)} ${padLeft(`${totals.nodeCompiled}/${totals.nodeTotal}`, 8)} ${padLeft(`${totals.compiled}/${totals.total}`, 10)}`,
    );
  }
  console.log('');
}

const opts = parseTopArgs(process.argv.slice(2));
if (opts.help) {
  printHelp();
  process.exit(0);
}

const targetIds = [...opts.targetSelections.keys()];
if (opts.listOnly) {
  printStatusTable(targetIds);
  process.exit(0);
}

assertTsrsProtoExists();

const startMs = Date.now();
for (const targetId of targetIds) {
  const sel = opts.targetSelections.get(targetId);
  const spec = TARGETS_BY_ID[targetId];
  const args = [path.join(ROOT, targetId, 'build.mjs')];
  if (opts.force) args.push('--force');
  if (opts.browserOnly) args.push('--browser');
  if (opts.jobsArg !== null) args.push('--jobs', opts.jobsArg);
  if (!sel.all) args.push(...sel.filters);

  console.log(`=== [${targetId}] ${spec.label} ===`);
  await spawnAsync(process.execPath, args, {
    cwd: path.join(ROOT, targetId),
    env: { ...process.env, MSBUILDDISABLENODEREUSE: '1' },
    buffered: false,
  });
}

const elapsedSec = ((Date.now() - startMs) / 1000).toFixed(1);
console.log(`All selected targets finished in ${elapsedSec}s.`);
printStatusTable(targetIds);
