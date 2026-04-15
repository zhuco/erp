import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import {
  createTaskCard,
  parseTaskCard,
  validateTaskFiles,
  readGitChangedFiles,
} from './task_card.mjs';

function createTempGitRepo() {
  const repoDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-ops-git-'));
  const run = (args) => {
    const result = spawnSync('git', args, {
      cwd: repoDir,
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, `git ${args.join(' ')} failed: ${result.stderr || result.stdout}`);
  };

  run(['init']);
  run(['config', 'user.email', 'codex@example.com']);
  run(['config', 'user.name', 'Codex']);

  fs.writeFileSync(path.join(repoDir, 'README.md'), '# temp\n', 'utf8');
  run(['add', 'README.md']);
  run(['commit', '-m', 'init']);

  return repoDir;
}

test('task card front matter round trips', () => {
  const card = createTaskCard({
    name: 'parallel-demo',
    goal: 'verify task card parser',
    background: 'for ops automation',
    owner: 'test-owner',
    branch: 'codex/demo',
    worktree: '.worktrees/demo',
    allowedFiles: ['scripts/ops/**', 'apps/backend/**'],
    disallowedFiles: ['apps/frontend/secret.md'],
    dependencies: ['shared lock registered'],
    dod: ['unit tests pass', 'boundary check pass'],
    testCommands: ['node --test scripts/ops/task_ops.test.mjs'],
    retryPolicy: 'transient-auto-retry(max=3,delay=10s)',
    fallbackStrategy: 'switch-to-local-execution',
    delivery: 'commit hash + test evidence + risk note',
  });

  const parsed = parseTaskCard(card);
  assert.equal(parsed.name, 'parallel-demo');
  assert.equal(parsed.goal, 'verify task card parser');
  assert.deepEqual(parsed.allowedFiles, ['scripts/ops/**', 'apps/backend/**']);
  assert.deepEqual(parsed.disallowedFiles, ['apps/frontend/secret.md']);
  assert.deepEqual(parsed.testCommands, ['node --test scripts/ops/task_ops.test.mjs']);
  assert.equal(parsed.retryPolicy, 'transient-auto-retry(max=3,delay=10s)');
  assert.equal(parsed.fallbackStrategy, 'switch-to-local-execution');
});

test('validateTaskFiles passes when all changes are in allowed scope', () => {
  const task = parseTaskCard(createTaskCard({
    name: 'allowlist-pass',
    goal: 'verify allowlist',
    owner: 'owner',
    branch: 'codex/pass',
    worktree: '.worktrees/pass',
    allowedFiles: ['scripts/ops/**', 'docs/ops/templates/**'],
    disallowedFiles: ['scripts/ops/secret/**'],
  }));

  const result = validateTaskFiles(task, [
    'scripts/ops/task_card.mjs',
    'docs/ops/templates/PARALLEL_TASK_CARD_TEMPLATE.md',
  ]);

  assert.equal(result.ok, true);
  assert.deepEqual(result.outOfScopeFiles, []);
  assert.deepEqual(result.blockedFiles, []);
});

test('validateTaskFiles fails on blocked and out-of-scope files', () => {
  const task = parseTaskCard(createTaskCard({
    name: 'boundary-fail',
    goal: 'verify disallowed paths',
    owner: 'owner',
    branch: 'codex/fail',
    worktree: '.worktrees/fail',
    allowedFiles: ['scripts/ops/**'],
    disallowedFiles: ['scripts/ops/private/**'],
  }));

  const result = validateTaskFiles(task, [
    'scripts/ops/private/token.txt',
    'apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx',
  ]);

  assert.equal(result.ok, false);
  assert.deepEqual(result.blockedFiles, ['scripts/ops/private/token.txt']);
  assert.deepEqual(result.outOfScopeFiles, ['apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx']);
});

test('new_parallel_task CLI generates task card with retry and fallback fields', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-card-'));
  const dest = path.join(tmpDir, 'task-card.md');
  const result = spawnSync(
    process.execPath,
    [
      'scripts/ops/new_parallel_task.mjs',
      '--name',
      'example-card',
      '--goal',
      'generate a card from cli',
      '--background',
      'for subtask resilience',
      '--owner',
      'cli-owner',
      '--branch',
      'codex/parallel',
      '--worktree',
      '.worktrees/codex-parallel',
      '--allowed',
      'scripts/ops/**',
      '--disallowed',
      'apps/erp-web/**',
      '--test',
      'node --test scripts/ops/task_ops.test.mjs',
      '--retry-policy',
      'transient-auto-retry(max=4,delay=15s)',
      '--fallback-strategy',
      'switch-to-local-execution',
      '--output',
      dest,
    ],
    {
      cwd: 'F:/erp',
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 0, `CLI exited with ${result.stderr || result.stdout}`);
  const contents = fs.readFileSync(dest, 'utf8');
  assert.ok(contents.includes('name: example-card'));
  assert.ok(contents.includes('goal: generate a card from cli'));
  assert.ok(contents.includes('allowed_files:'));
  assert.ok(contents.includes('retry_policy: transient-auto-retry(max=4,delay=15s)'));
  assert.ok(contents.includes('fallback_strategy: switch-to-local-execution'));
  assert.ok(contents.includes('## Test Commands'));
});

test('validate_task_boundary CLI supports explicit file list', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-boundary-'));
  const taskPath = path.join(tmpDir, 'task-card.md');
  fs.writeFileSync(taskPath, createTaskCard({
    name: 'explicit-file-validation',
    goal: 'verify validator cli',
    owner: 'owner',
    branch: 'codex/validate',
    worktree: '.worktrees/validate',
    allowedFiles: ['scripts/ops/**'],
    disallowedFiles: ['scripts/ops/private/**'],
  }), 'utf8');

  const passResult = spawnSync(
    process.execPath,
    [
      'scripts/ops/validate_task_boundary.mjs',
      '--task',
      taskPath,
      '--file',
      'scripts/ops/task_card.mjs',
    ],
    {
      cwd: 'F:/erp',
      encoding: 'utf8',
    },
  );
  assert.equal(passResult.status, 0, passResult.stderr || passResult.stdout);
  assert.ok(passResult.stdout.includes('PASS'));

  const failResult = spawnSync(
    process.execPath,
    [
      'scripts/ops/validate_task_boundary.mjs',
      '--task',
      taskPath,
      '--file',
      'apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx',
    ],
    {
      cwd: 'F:/erp',
      encoding: 'utf8',
    },
  );
  assert.equal(failResult.status, 1);
  assert.ok(failResult.stdout.includes('FAIL'));
});

test('readGitChangedFiles supports staged, working, all modes', () => {
  const repoDir = createTempGitRepo();
  const stagedFile = path.join(repoDir, 'scripts', 'ops', 'staged.txt');
  const workingFile = path.join(repoDir, 'docs', 'ops', 'working.txt');

  fs.mkdirSync(path.dirname(stagedFile), { recursive: true });
  fs.mkdirSync(path.dirname(workingFile), { recursive: true });
  fs.writeFileSync(stagedFile, 'staged\n', 'utf8');
  fs.writeFileSync(workingFile, 'working\n', 'utf8');

  const addResult = spawnSync('git', ['add', 'scripts/ops/staged.txt'], {
    cwd: repoDir,
    encoding: 'utf8',
  });
  assert.equal(addResult.status, 0, addResult.stderr || addResult.stdout);

  const stagedFiles = readGitChangedFiles({ repoRoot: repoDir, mode: 'staged' });
  const workingFiles = readGitChangedFiles({ repoRoot: repoDir, mode: 'working' });
  const allFiles = readGitChangedFiles({ repoRoot: repoDir, mode: 'all' });

  assert.deepEqual(stagedFiles, ['scripts/ops/staged.txt']);
  assert.deepEqual(workingFiles, ['docs/ops/working.txt']);
  assert.deepEqual(allFiles, ['docs/ops/working.txt', 'scripts/ops/staged.txt']);
});
