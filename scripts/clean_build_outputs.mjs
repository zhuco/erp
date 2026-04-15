import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function removeDirIfExists(dirPath) {
  if (!fs.existsSync(dirPath)) {
    return false;
  }
  fs.rmSync(dirPath, { recursive: true, force: true });
  return true;
}

const removed = [];

for (const rel of ['build', 'target']) {
  const abs = path.resolve(root, rel);
  if (removeDirIfExists(abs)) {
    removed.push(abs);
  }
}

const webDir = path.resolve(root, 'apps/erp-web');
if (fs.existsSync(webDir)) {
  const staticDirs = ['test-results', 'playwright-report', 'node_modules/.vite'];
  for (const rel of staticDirs) {
    const abs = path.resolve(webDir, rel);
    if (removeDirIfExists(abs)) {
      removed.push(abs);
    }
  }

  const entries = fs.readdirSync(webDir, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    if (entry.name === 'dist' || entry.name.startsWith('dist-')) {
      const abs = path.resolve(webDir, entry.name);
      if (removeDirIfExists(abs)) {
        removed.push(abs);
      }
    }
  }
}

if (removed.length === 0) {
  console.log('[build:clean] no build directories found');
} else {
  console.log('[build:clean] removed directories:');
  for (const dir of removed) {
    console.log(`- ${dir}`);
  }
}
