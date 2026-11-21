import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createTargetViteConfig } from '../targets.mjs';

const ROOT = path.dirname(fileURLToPath(import.meta.url));
export default createTargetViteConfig(ROOT, 'fsharp');
