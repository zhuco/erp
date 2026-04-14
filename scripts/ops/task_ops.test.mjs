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
    name: '并行任务示例',
    goal: '校验卡片',
    background: '用于示范',
    owner: 'test-owner',
    branch: 'codex/demo',
    worktree: '.worktrees/demo',
    allowedFiles: ['scripts/ops/**', 'apps/backend/**'],
    disallowedFiles: ['apps/frontend/secret.md'],
    dependencies: ['共享文件锁已登记'],
    dod: ['通过单测', '通过边界校验'],
    testCommands: ['node --test scripts/ops/task_ops.test.mjs'],
    delivery: 'commit hash + 测试结果 + 风险',
  });

  const parsed = parseTaskCard(card);
  assert.equal(parsed.name, '并行任务示例');
  assert.equal(parsed.goal, '校验卡片');
  assert.deepEqual(parsed.allowedFiles, ['scripts/ops/**', 'apps/backend/**']);
  assert.deepEqual(parsed.disallowedFiles, ['apps/frontend/secret.md']);
  assert.deepEqual(parsed.testCommands, ['node --test scripts/ops/task_ops.test.mjs']);
});

test('validateTaskFiles 在白名单范围内时 PASS', () => {
  const task = parseTaskCard(createTaskCard({
    name: '白名单通过',
    goal: '验证白名单',
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

test('validateTaskFiles 命中禁区或越界时 FAIL', () => {
  const task = parseTaskCard(createTaskCard({
    name: '边界失败',
    goal: '验证禁区',
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

test('new_parallel_task CLI 生成包含核心字段的任务卡', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-card-'));
  const dest = path.join(tmpDir, 'task-card.md');
  const result = spawnSync(
    process.execPath,
    [
      'scripts/ops/new_parallel_task.mjs',
      '--name',
      '示例卡片',
      '--goal',
      '落地任务卡脚手架',
      '--background',
      '用于售后多 AI 并行',
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
  assert.ok(contents.includes('name: 示例卡片'));
  assert.ok(contents.includes('goal: 落地任务卡脚手架'));
  assert.ok(contents.includes('allowed_files:'));
  assert.ok(contents.includes('## 测试命令'));
});

test('validate_task_boundary CLI 支持显式文件列表', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-boundary-'));
  const taskPath = path.join(tmpDir, 'task-card.md');
  fs.writeFileSync(taskPath, createTaskCard({
    name: '显式文件边界校验',
    goal: '验证 CLI',
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

test('readGitChangedFiles 支持 staged / working / all 三种模式', () => {
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
