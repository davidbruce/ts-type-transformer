import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runTargetBuild, spawnAsync } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));

await runTargetBuild({
  targetId: 'zig',
  root: ROOT,
  domSentinel: 'bindings.zig',
  pkgSentinel: 'glue.mjs',
  extraFiles: ['build.zig'],
  srcSubdir: 'src',
  buildDemo: async ({ demoDir, outPath, buffered }) => {
    await spawnAsync('zig', ['build', '-Drelease=true'], { cwd: demoDir, buffered });
    fs.copyFileSync(path.join(demoDir, 'zig-out/bin/app.wasm'), outPath);
  },
});
