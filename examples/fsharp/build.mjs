import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runTargetBuild, spawnAsync } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));

const fableBin =
  [
    path.join(os.homedir(), '.dotnet/tools/fable'),
    path.join(os.homedir(), '.dotnet/tools/fable.exe'),
  ].find(p => fs.existsSync(p)) ?? 'fable';

await runTargetBuild({
  targetId: 'fsharp',
  root: ROOT,
  domDirName: 'Dom',
  domSentinel: 'Dom.fsproj',
  pkgSentinel: spec => `${spec.outDir}.fsproj`,
  extraFiles: ['App.fsproj'],
  bindingExt: '.fs',
  buildDemo: ({ demoDir, buffered }) => {
    const fableArgs = ['App.fsproj', '-o', 'js', '--extension', '.mjs'];
    if (process.env.FABLE_NO_CACHE === '1') fableArgs.push('--noCache');
    return spawnAsync(fableBin, fableArgs, {
      cwd: demoDir,
      buffered,
      env: { ...process.env, MSBUILDDISABLENODEREUSE: '1' },
    });
  },
});
