import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  TARGETS,
  TARGETS_BY_ID,
  VITE_OPTIMIZE_EXCLUDE,
  VITE_WATCH_IGNORED,
  createFallbackLibrariesPlugin,
  createQueuedDemoBuilder,
  getTargetStatus,
  normalizeHtmlPathname,
  readManifest,
  sendJson,
} from './targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const FALLBACK_PKG_JSON = path.resolve(ROOT, '../target/libraries/package.json');
const FALLBACK_NODE_MODULES = path.resolve(ROOT, '../target/libraries/node_modules');

const manifests = new Map();
function getManifest(targetId) {
  let m = manifests.get(targetId);
  if (!m) {
    m = readManifest(path.join(ROOT, targetId));
    manifests.set(targetId, m);
  }
  return m;
}

const builder = createQueuedDemoBuilder(ROOT);

export default {
  root: ROOT,
  appType: 'mpa',
  optimizeDeps: {
    exclude: VITE_OPTIMIZE_EXCLUDE,
    entries: [],
  },
  server: {
    fs: {
      allow: [ROOT, path.resolve(ROOT, '..')],
    },
    watch: {
      ignored: VITE_WATCH_IGNORED,
    },
  },
  plugins: [
    createFallbackLibrariesPlugin(FALLBACK_NODE_MODULES, FALLBACK_PKG_JSON),
    {
      name: 'examples-status-and-lazy-build',
      configureServer(server) {
        server.middlewares.use(async (req, res, next) => {
          const url = new URL(req.url ?? '/', 'http://localhost');
          if (req.method === 'GET' && url.pathname === '/__api/status') {
            const targetId = url.searchParams.get('target');
            if (targetId) {
              if (!TARGETS_BY_ID[targetId]) {
                return sendJson(res, 400, { error: `Unknown target: ${targetId}` });
              }
              return sendJson(res, 200, { target: getTargetStatus(ROOT, targetId) });
            }
            return sendJson(res, 200, { targets: TARGETS.map(t => getTargetStatus(ROOT, t.id)) });
          }

          const pathname = normalizeHtmlPathname(req.url);
          const segs = pathname.replace(/^\/+/, '').split('/');
          if (segs.length < 2) return next();
          const [targetId, ...rest] = segs;
          if (!TARGETS_BY_ID[targetId]) return next();

          const rel = `/${rest.join('/')}`;
          const manifest = getManifest(targetId);
          const item = manifest.examples.find(
            e => e.kind !== 'node' && `/${e.path.replace(/^\.\//, '')}` === rel,
          );
          if (!item) return next();

          await builder.ensureBuiltAndServe(server, res, next, targetId, item);
        });
      },
    },
  ],
};
