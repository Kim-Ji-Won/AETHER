import { execFileSync } from 'node:child_process';
import { copyFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

const desktop = fileURLToPath(new URL('..', import.meta.url));
const out = resolve(desktop, 'src-tauri/sidecar/generated');
execFileSync('python3', [resolve(desktop, '../../scripts/prepare_node_sidecar.py'), '--verify-only'], { stdio: 'inherit' });
// Compile the exact TypeScript module used by WebView A; no duplicate fixture implementation.
execFileSync(resolve(desktop, 'node_modules/.bin/tsc'), [
  '--ignoreConfig', 'src/fake-provider.ts', '--outDir', out, '--target', 'ES2022',
  '--module', 'ESNext', '--skipLibCheck', '--noEmit', 'false',
], { cwd: desktop, stdio: 'inherit' });
copyFileSync(resolve(desktop, 'sidecar/entry.mjs'), resolve(out, 'entry.mjs'));
writeFileSync(resolve(out, 'package.json'), '{"type":"module"}\n');
