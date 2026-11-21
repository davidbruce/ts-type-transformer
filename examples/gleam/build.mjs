import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runTargetBuild, spawnAsync } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
const STDLIB_CHECKSUM = 'CEE5B6C076A85B45F60C585F4316C63EC8B7127C119D5738C3958A9C4D50404E';

await runTargetBuild({
  targetId: 'gleam',
  root: ROOT,
  domSentinel: 'gleam.toml',
  pkgSentinel: 'gleam.toml',
  extraFiles: ['gleam.toml'],
  srcSubdir: 'src',
  buildDemo: ({ demoDir, specs, buffered }) => {
    const pkgEntries = [];
    const reqEntries = [`gleam_stdlib = { version = ">= 1.0.0 and < 2.0.0" }`];
    const needsDom = specs.some(s => s.needsDom);
    const seen = new Set();
    for (const s of specs) {
      if (seen.has(s.outDir)) continue;
      seen.add(s.outDir);
      const reqList = s.needsDom ? `["dom", "gleam_stdlib"]` : `["gleam_stdlib"]`;
      pkgEntries.push(
        `  { name = "${s.outDir}", version = "0.1.0", build_tools = ["gleam"], requirements = ${reqList}, source = "local", path = "../../bindings/${s.outDir}" },`,
      );
      reqEntries.unshift(`${s.outDir} = { path = "../../bindings/${s.outDir}" }`);
    }
    if (needsDom) {
      pkgEntries.push(
        `  { name = "dom", version = "0.1.0", build_tools = ["gleam"], requirements = ["gleam_stdlib"], source = "local", path = "../../bindings/dom" },`,
      );
      reqEntries.unshift(`dom = { path = "../../bindings/dom" }`);
    }
    pkgEntries.push(
      `  { name = "gleam_stdlib", version = "1.0.5", build_tools = ["gleam"], requirements = [], otp_app = "gleam_stdlib", source = "hex", outer_checksum = "${STDLIB_CHECKSUM}" },`,
    );
    fs.writeFileSync(
      path.join(demoDir, 'manifest.toml'),
      `packages = [\n${pkgEntries.join('\n')}\n]\n\n[requirements]\n${reqEntries.join('\n')}\n`,
    );
    return spawnAsync('gleam', ['build', '--warnings-as-errors'], { cwd: demoDir, buffered });
  },
});
