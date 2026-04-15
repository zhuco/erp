import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createTaskCard } from './task_card.mjs';

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..', '..');

const LIST_FLAGS = {
  '--allowed': 'allowedFiles',
  '--disallowed': 'disallowedFiles',
  '--shared-lock-target': 'sharedLockTargets',
  '--shared-lock-targets': 'sharedLockTargets',
  '--dependency': 'dependencies',
  '--dependencies': 'dependencies',
  '--dod': 'dod',
  '--test': 'testCommands',
  '--test-command': 'testCommands',
  '--test-commands': 'testCommands',
};

const SINGLE_FLAGS = {
  '--name': 'name',
  '--goal': 'goal',
  '--background': 'background',
  '--owner': 'owner',
  '--window': 'window',
  '--domain': 'domain',
  '--slot': 'slot',
  '--branch': 'branch',
  '--worktree': 'worktree',
  '--delivery': 'delivery',
  '--submission-format': 'submissionFormat',
  '--output': 'output',
};

const USAGE = `Usage: node new_parallel_task.mjs --name NAME --goal GOAL --owner OWNER --branch BRANCH --worktree WORKTREE [options]
Options:
  --background TEXT
  --window Wn
  --domain DOMAIN
  --slot T1|T2|T3
  --delivery TEXT
  --submission-format TEXT
  --allowed PATH
  --disallowed PATH
  --shared-lock-target PATH_OR_GLOB
  --dependency TEXT
  --dod ITEM
  --test COMMAND
  --output PATH (default: task-card.md)
`;

function exitWithError(message) {
  console.error(`[new_parallel_task] ${message}`);
  printUsage();
  process.exit(1);
}

function printUsage() {
  console.error(USAGE);
}

const args = process.argv.slice(2);
if (!args.length || args.includes('--help')) {
  printUsage();
  process.exit(args.includes('--help') ? 0 : 1);
}

const parsed = {
  allowedFiles: [],
  disallowedFiles: [],
  sharedLockTargets: [],
  dependencies: [],
  dod: [],
  testCommands: [],
};

for (let i = 0; i < args.length; i += 1) {
  const flag = args[i];
  if (LIST_FLAGS[flag]) {
    const next = args[i + 1];
    if (!next || next.startsWith('--')) {
      exitWithError(`missing value for ${flag}`);
    }
    const listKey = LIST_FLAGS[flag];
    parsed[listKey].push(next);
    i += 1;
    continue;
  }
  if (SINGLE_FLAGS[flag]) {
    const next = args[i + 1];
    if (!next || next.startsWith('--')) {
      exitWithError(`missing value for ${flag}`);
    }
    parsed[SINGLE_FLAGS[flag]] = next;
    i += 1;
    continue;
  }
  exitWithError(`unknown argument ${flag}`);
}

const required = ['name', 'goal', 'owner', 'branch', 'worktree'];
for (const key of required) {
  if (!parsed[key]) {
    exitWithError(`missing required argument --${key}`);
  }
}

const metadata = {
  name: parsed.name,
  goal: parsed.goal,
  background: parsed.background,
  owner: parsed.owner,
  window: parsed.window,
  domain: parsed.domain,
  slot: parsed.slot,
  branch: parsed.branch,
  worktree: parsed.worktree,
  allowedFiles: parsed.allowedFiles,
  disallowedFiles: parsed.disallowedFiles,
  sharedLockTargets: parsed.sharedLockTargets,
  dependencies: parsed.dependencies,
  dod: parsed.dod,
  testCommands: parsed.testCommands,
  submissionFormat: parsed.submissionFormat,
  delivery: parsed.delivery,
};

const outputPath = path.resolve(repoRoot, parsed.output || 'task-card.md');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, createTaskCard(metadata), 'utf8');
console.log(`[new_parallel_task] wrote ${outputPath}`);
