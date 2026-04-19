import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const repoRoot = process.cwd();
const args = new Set(process.argv.slice(2));
const stagedOnly = args.has('--staged');

const decoder = new TextDecoder('utf-8', { fatal: true });
const suspiciousMojibakePattern = new RegExp(
  '[\\u95b9\\u95b8\\u95ba\\u5a62\\u7f02\\u5a23\\u6960\\u93bc\\u9420\\u95c1\\u941f\\u93c9\\u5a34\\u6fde\\u940e]{2,}',
  'u'
);

const binaryExtensions = new Set([
  '.avif',
  '.bmp',
  '.class',
  '.dll',
  '.dylib',
  '.eot',
  '.exe',
  '.gif',
  '.gz',
  '.ico',
  '.jar',
  '.jpeg',
  '.jpg',
  '.mp3',
  '.mp4',
  '.otf',
  '.pdf',
  '.png',
  '.so',
  '.svgz',
  '.tar',
  '.ttf',
  '.wasm',
  '.webm',
  '.webp',
  '.woff',
  '.woff2',
  '.zip',
]);

function readGitFileList() {
  const commandArgs = stagedOnly
    ? ['diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z']
    : ['ls-files', '-z'];
  const output = execFileSync('git', commandArgs, { cwd: repoRoot, encoding: 'buffer' });
  return output
    .toString('utf8')
    .split('\0')
    .map((item) => item.trim())
    .filter(Boolean);
}

function hasNullByte(buffer) {
  return buffer.includes(0);
}

function isLikelyBinary(filePath, buffer) {
  if (binaryExtensions.has(path.extname(filePath).toLowerCase())) {
    return true;
  }
  return hasNullByte(buffer.slice(0, Math.min(buffer.length, 4096)));
}

function hasUtf8Bom(buffer) {
  return (
    buffer.length >= 3
    && buffer[0] === 0xef
    && buffer[1] === 0xbb
    && buffer[2] === 0xbf
  );
}

function isValidUtf8(buffer) {
  try {
    decoder.decode(buffer);
    return true;
  } catch {
    return false;
  }
}

function collectMojibakeFindings(text, maxFindings = 5) {
  const findings = [];
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (suspiciousMojibakePattern.test(line)) {
      findings.push({
        line: index + 1,
        snippet: line.trim().slice(0, 64),
      });
      if (findings.length >= maxFindings) {
        return findings;
      }
    }
  }
  return findings;
}

function normalizeForOutput(filePath) {
  return filePath.replace(/\\/g, '/');
}

const files = readGitFileList();
const invalidUtf8Files = [];
const bomFiles = [];
const mojibakeFiles = [];
let checkedCount = 0;

for (const file of files) {
  const absolutePath = path.join(repoRoot, file);
  if (!fs.existsSync(absolutePath) || !fs.statSync(absolutePath).isFile()) {
    continue;
  }

  const buffer = fs.readFileSync(absolutePath);
  if (buffer.length === 0 || isLikelyBinary(file, buffer)) {
    continue;
  }

  checkedCount += 1;

  if (!isValidUtf8(buffer)) {
    invalidUtf8Files.push(normalizeForOutput(file));
    continue;
  }

  const text = buffer.toString('utf8');
  const findings = collectMojibakeFindings(text);
  if (findings.length > 0) {
    mojibakeFiles.push({
      file: normalizeForOutput(file),
      findings,
    });
  }

  if (hasUtf8Bom(buffer)) {
    bomFiles.push(normalizeForOutput(file));
  }
}

if (invalidUtf8Files.length > 0 || bomFiles.length > 0 || mojibakeFiles.length > 0) {
  console.error('[utf8-check] Failed.');

  if (invalidUtf8Files.length > 0) {
    console.error('[utf8-check] Invalid UTF-8 files:');
    for (const file of invalidUtf8Files) {
      console.error(`  - ${file}`);
    }
  }

  if (bomFiles.length > 0) {
    console.error('[utf8-check] UTF-8 BOM is not allowed in source files:');
    for (const file of bomFiles) {
      console.error(`  - ${file}`);
    }
  }

  if (mojibakeFiles.length > 0) {
    console.error('[utf8-check] Suspected Chinese mojibake content:');
    for (const item of mojibakeFiles) {
      console.error(`  - ${item.file}`);
      for (const finding of item.findings) {
        console.error(`    line ${finding.line}: ${finding.snippet}`);
      }
    }
  }

  process.exit(1);
}

const scopeLabel = stagedOnly ? 'staged files' : 'tracked files';
console.log(`[utf8-check] OK. Checked ${checkedCount} ${scopeLabel}.`);
