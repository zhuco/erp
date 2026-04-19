import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..');

const gitDir = path.join(repoRoot, '.git');
if (!fs.existsSync(gitDir)) {
  console.log('[hooks] Skipped: .git directory not found.');
  process.exit(0);
}

try {
  execFileSync('git', ['config', 'core.hooksPath', '.githooks'], {
    cwd: repoRoot,
    stdio: 'inherit',
  });
  console.log('[hooks] Installed: core.hooksPath=.githooks');
} catch (error) {
  console.warn('[hooks] Failed to install git hooks automatically.');
  console.warn(`[hooks] ${error instanceof Error ? error.message : String(error)}`);
  process.exit(0);
}
