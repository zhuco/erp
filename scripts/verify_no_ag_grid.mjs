import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sourceRoots = ['apps', 'crates', 'packages'];
const sourceExtensions = new Set([
  '.cjs', '.css', '.html', '.js', '.jsx', '.mjs', '.rs', '.scss', '.ts', '.tsx',
]);
const excludedDirectories = new Set([
  '.git', '.playwright-cli', 'build', 'coverage', 'dist', 'docs', 'node_modules', 'qa',
  'pkg', 'target', 'tmp',
]);
const manifestNames = new Set([
  'Cargo.toml', 'Cargo.lock', 'package.json', 'package-lock.json', 'pnpm-lock.yaml', 'yarn.lock',
]);
const forbidden = /@ag-grid-|\bag[ -]grid\b|\bAgGrid[A-Za-z0-9_]*|\bagGrid[A-Za-z0-9_]*|\bag-(?:theme|root|body|header|cell|row|center|pinned)(?:-|\b)/i;

function walk(directory, include) {
  if (!fs.existsSync(directory)) return [];
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && excludedDirectories.has(entry.name)) continue;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...walk(absolute, include));
    else if (entry.isFile() && include(absolute, entry.name)) files.push(absolute);
  }
  return files;
}

const candidates = new Set();
for (const root of sourceRoots) {
  for (const file of walk(path.join(repo, root), (absolute) => sourceExtensions.has(path.extname(absolute)))) {
    candidates.add(file);
  }
}
for (const file of walk(repo, (_absolute, name) => manifestNames.has(name))) candidates.add(file);

const hits = [];
for (const file of [...candidates].sort()) {
  const relative = path.relative(repo, file).replaceAll(path.sep, '/');
  for (const [index, line] of fs.readFileSync(file, 'utf8').split(/\r?\n/).entries()) {
    if (forbidden.test(line)) hits.push(`${relative}:${index + 1}`);
  }
}

if (hits.length) {
  const shown = hits.slice(0, 100);
  const remainder = hits.length - shown.length;
  console.error(`禁止 AG Grid：发现 ${hits.length} 处源码、清单或锁文件引用。`);
  console.error(shown.join('\n'));
  if (remainder) console.error(`另有 ${remainder} 处未显示。`);
  process.exit(1);
}

console.log(`AG Grid 禁止门禁通过（扫描 ${candidates.size} 个源码、清单及锁文件）。`);
