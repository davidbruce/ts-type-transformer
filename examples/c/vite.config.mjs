import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const FALLBACK_PKG_JSON = path.resolve(ROOT, '../../target/libraries/package.json');
const FALLBACK_NODE_MODULES = path.resolve(ROOT, '../../target/libraries/node_modules');

function readManifest() {
  return JSON.parse(fs.readFileSync(path.join(ROOT, 'examples.json'), 'utf8'));
}

export default {
  appType: 'mpa',
  server: {
    fs: {
      allow: [ROOT, path.resolve(ROOT, '../..')],
    },
  },
  plugins: [
    {
      name: 'fallback-libraries-node-modules',
      async resolveId(id, importer) {
        if (
          !fs.existsSync(FALLBACK_NODE_MODULES) ||
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
        return this.resolve(id, FALLBACK_PKG_JSON, { skipSelf: true });
      },
    },
    {
      name: 'auto-build-examples',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          const rawUrl = (req.url ?? '').split('?')[0].split('#')[0];
          const manifest = readManifest();
          const item = manifest.examples.find(
            e => e.kind !== 'node' && `/${e.path.replace(/^\.\//, '')}` === rawUrl,
          );
          if (!item) return next();
          const demoDir = path.join(ROOT, path.dirname(item.path));
          const artifact = path.join(demoDir, 'app.wasm');
          if (!fs.existsSync(artifact)) {
            try {
              execFileSync(process.execPath, [path.join(ROOT, 'build.mjs'), item.id], {
                cwd: ROOT,
                stdio: 'inherit',
                env: { ...process.env, MSBUILDDISABLENODEREUSE: '1' },
              });
            } catch (err) {
              res.statusCode = 500;
              res.setHeader('Content-Type', 'text/html; charset=utf-8');
              const msg = String(err?.message ?? err)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;');
              res.end(
                `<!doctype html><html><body style="font-family:monospace;padding:20px;background:#fff;color:#111"><h1>Build failed</h1><pre>${msg}</pre></body></html>`,
              );
              return;
            }
          }
          next();
        });
      },
    },
  ],
};
