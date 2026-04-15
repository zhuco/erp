import fs from 'node:fs';
import path from 'node:path';
import {
  matchesAnyPattern,
  normalizePath,
  parseTaskCard,
  readGitChangedFiles,
  validateTaskFiles,
} from './task_card.mjs';

const LIST_FLAGS = {
  '--file': 'files',
};

const SINGLE_FLAGS = {
  '--task': 'task',
  '--mode': 'mode',
  '--policy-file': 'policyFile',
  '--lock-registry': 'lockRegistry',
};

const USAGE = `Usage: node validate_task_boundary.mjs --task PATH [--file PATH ...] [--mode working|staged|all] [--policy-file PATH] [--lock-registry PATH]
Options:
  --task PATH           task card markdown path
  --file PATH           explicit changed file list (repeatable)
  --mode MODE           when --file absent: working|staged|all (default: working)
  --policy-file PATH    shared lock policy json path
  --lock-registry PATH  shared lock registry json path
`;

function printUsage(exitCode = 0) {
  process.stdout.write(`${USAGE}\n`);
  process.exit(exitCode);
}

function fail(message, details = []) {
  process.stdout.write(`FAIL: ${message}\n`);
  for (const detail of details) {
    process.stdout.write(`- ${detail}\n`);
  }
  process.exit(1);
}

function loadJsonFile(filePath, fallback) {
  if (!fs.existsSync(filePath)) {
    return fallback;
  }

  const text = fs.readFileSync(filePath, 'utf8').trim();
  if (!text) {
    return fallback;
  }

  return JSON.parse(text);
}

function validateSharedLocks({ checkedFiles, taskCard, taskPath, policy, registry }) {
  const lockedPatterns = Array.isArray(policy?.locked_patterns) ? policy.locked_patterns : [];
  if (!lockedPatterns.length) {
    return {
      ok: true,
      lockedFiles: [],
      missingDeclarations: [],
      missingLocks: [],
      conflictingLocks: [],
    };
  }

  const lockedFiles = checkedFiles.filter((filePath) => matchesAnyPattern(filePath, lockedPatterns));
  if (!lockedFiles.length) {
    return {
      ok: true,
      lockedFiles: [],
      missingDeclarations: [],
      missingLocks: [],
      conflictingLocks: [],
    };
  }

  const owner = String(taskCard.owner || '').trim();
  const normalizedTaskPath = normalizePath(taskPath);
  const declaredTargets = Array.isArray(taskCard.sharedLockTargets) ? taskCard.sharedLockTargets : [];
  const activeLocks = Array.isArray(registry?.locks) ? registry.locks : [];

  const missingDeclarations = [];
  const missingLocks = [];
  const conflictingLocks = [];

  for (const filePath of lockedFiles) {
    const hasDeclaration = declaredTargets.some((pattern) => matchesAnyPattern(filePath, [pattern]));
    if (!hasDeclaration) {
      missingDeclarations.push(filePath);
    }

    const ownedLock = activeLocks.find((lock) => {
      const pattern = String(lock?.pattern || '');
      const lockOwner = String(lock?.owner || '');
      const lockTask = normalizePath(String(lock?.task || ''));
      if (!pattern) {
        return false;
      }
      return lockOwner === owner && lockTask === normalizedTaskPath && matchesAnyPattern(filePath, [pattern]);
    });

    if (!ownedLock) {
      missingLocks.push(filePath);
    }

    const conflictLock = activeLocks.find((lock) => {
      const pattern = String(lock?.pattern || '');
      const lockTask = normalizePath(String(lock?.task || ''));
      if (!pattern || lockTask === normalizedTaskPath) {
        return false;
      }
      return matchesAnyPattern(filePath, [pattern]);
    });

    if (conflictLock) {
      conflictingLocks.push(`${filePath} <- ${conflictLock.task} (${conflictLock.owner || 'unknown'})`);
    }
  }

  return {
    ok: missingDeclarations.length === 0 && missingLocks.length === 0 && conflictingLocks.length === 0,
    lockedFiles,
    missingDeclarations,
    missingLocks,
    conflictingLocks,
  };
}

const args = process.argv.slice(2);
if (!args.length || args.includes('--help')) {
  printUsage(args.includes('--help') ? 0 : 1);
}

const parsed = {
  files: [],
  mode: 'working',
};

for (let i = 0; i < args.length; i += 1) {
  const flag = args[i];
  if (LIST_FLAGS[flag]) {
    const next = args[i + 1];
    if (!next || next.startsWith('--')) {
      fail(`missing value for ${flag}`);
    }
    parsed[LIST_FLAGS[flag]].push(next);
    i += 1;
    continue;
  }
  if (SINGLE_FLAGS[flag]) {
    const next = args[i + 1];
    if (!next || next.startsWith('--')) {
      fail(`missing value for ${flag}`);
    }
    parsed[SINGLE_FLAGS[flag]] = next;
    i += 1;
    continue;
  }
  fail(`unknown argument ${flag}`);
}

if (!parsed.task) {
  fail('missing required argument --task');
}

const repoRoot = process.cwd();
const policyPath = path.resolve(parsed.policyFile || path.join(repoRoot, 'docs', 'ops', 'config', 'PARALLEL_SHARED_LOCKS_V2.json'));
const registryPath = path.resolve(parsed.lockRegistry || path.join(repoRoot, 'docs', 'ops', 'state', 'PARALLEL_SHARED_LOCK_REGISTRY.json'));
const taskPath = path.resolve(parsed.task);

if (!fs.existsSync(taskPath)) {
  fail(`task card not found: ${taskPath}`);
}

const cardText = fs.readFileSync(taskPath, 'utf8');
const taskCard = parseTaskCard(cardText);
const files = parsed.files.length
  ? parsed.files
  : readGitChangedFiles({ repoRoot, mode: parsed.mode });

if (!files.length) {
  process.stdout.write('PASS: no changed files to validate\n');
  process.exit(0);
}

const result = validateTaskFiles(taskCard, files);
if (!result.ok) {
  const details = [];
  if (result.blockedFiles.length) {
    details.push(`blocked files: ${result.blockedFiles.join(', ')}`);
  }
  if (result.outOfScopeFiles.length) {
    details.push(`out of scope files: ${result.outOfScopeFiles.join(', ')}`);
  }
  fail('task boundary validation failed', details);
}

const policy = loadJsonFile(policyPath, { locked_patterns: [] });
const registry = loadJsonFile(registryPath, { locks: [] });
const sharedLockResult = validateSharedLocks({
  checkedFiles: result.checkedFiles,
  taskCard,
  taskPath,
  policy,
  registry,
});

if (!sharedLockResult.ok) {
  const details = [];
  if (sharedLockResult.missingDeclarations.length) {
    details.push(`shared lock declaration missing for: ${sharedLockResult.missingDeclarations.join(', ')}`);
  }
  if (sharedLockResult.missingLocks.length) {
    details.push(`shared lock not acquired for: ${sharedLockResult.missingLocks.join(', ')}`);
  }
  if (sharedLockResult.conflictingLocks.length) {
    details.push(`shared lock conflict: ${sharedLockResult.conflictingLocks.join(', ')}`);
  }
  fail('shared lock validation failed', details);
}

process.stdout.write(`PASS: validated ${result.checkedFiles.length} files within task scope`);
if (sharedLockResult.lockedFiles.length) {
  process.stdout.write(`; shared lock verified for ${sharedLockResult.lockedFiles.length} file(s)`);
}
process.stdout.write('\n');
