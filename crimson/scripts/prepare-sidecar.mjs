// Builds the `crimson-server` sidecar and copies it into `src-tauri/bin` using
// the target-triple naming Tauri expects for `externalBin` ("bin/crimson-server"
// -> "bin/crimson-server-<triple>[.exe]"). This lets `tauri dev`/`tauri build`
// find the sidecar on any OS, so launching the desktop app is a single command
// on Linux/macOS/Windows alike.
//
// Usage: node scripts/prepare-sidecar.mjs [--release]
import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, copyFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..'); // crimson/scripts -> repo root
const release = process.argv.includes('--release');
const profile = release ? 'release' : 'debug';
const exe = process.platform === 'win32' ? '.exe' : '';

// Determine the host target triple (matches what Tauri appends to externalBin).
const rustcVerbose = execSync('rustc -vV', { encoding: 'utf8' });
const triple = (rustcVerbose.match(/^host:\s*(.+)$/m) || [])[1];
if (!triple) {
  console.error('[prepare-sidecar] could not determine host triple from `rustc -vV`.');
  process.exit(1);
}

console.log(`[prepare-sidecar] building crimson-server (${profile}) for ${triple}…`);
execSync(`cargo build -p crimson-server${release ? ' --release' : ''}`, {
  cwd: repoRoot,
  stdio: 'inherit',
});

const built = resolve(repoRoot, 'target', profile, `crimson-server${exe}`);
if (!existsSync(built)) {
  console.error(`[prepare-sidecar] built binary not found at ${built}`);
  process.exit(1);
}

const binDir = resolve(here, '..', 'src-tauri', 'bin');
mkdirSync(binDir, { recursive: true });
const dest = resolve(binDir, `crimson-server-${triple}${exe}`);
copyFileSync(built, dest);
console.log(`[prepare-sidecar] copied -> ${dest}`);
