import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  createTaskCard,
  parseTaskCard,
  validateTaskFiles,
  readGitChangedFiles,
} from './task_card.mjs';

const currentFilePath = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(currentFilePath), '..', '..');

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
    name: '骞惰浠诲姟绀轰緥',
    goal: '鏍￠獙鍗＄墖',
    background: '鐢ㄤ簬绀鸿寖',
    owner: 'test-owner',
    branch: 'codex/demo',
    worktree: '.worktrees/demo',
    allowedFiles: ['scripts/ops/**', 'apps/backend/**'],
    disallowedFiles: ['apps/frontend/secret.md'],
    dependencies: ['鍏变韩鏂囦欢閿佸凡鐧昏'],
    dod: ['閫氳繃鍗曟祴', '閫氳繃杈圭晫鏍￠獙'],
    testCommands: ['node --test scripts/ops/task_ops.test.mjs'],
    delivery: 'commit hash + 娴嬭瘯缁撴灉 + 椋庨櫓',
  });

  const parsed = parseTaskCard(card);
  assert.equal(parsed.name, '骞惰浠诲姟绀轰緥');
  assert.equal(parsed.goal, '鏍￠獙鍗＄墖');
  assert.deepEqual(parsed.allowedFiles, ['scripts/ops/**', 'apps/backend/**']);
  assert.deepEqual(parsed.disallowedFiles, ['apps/frontend/secret.md']);
  assert.deepEqual(parsed.testCommands, ['node --test scripts/ops/task_ops.test.mjs']);
});

test('validateTaskFiles 鍦ㄧ櫧鍚嶅崟鑼冨洿鍐呮椂 PASS', () => {
  const task = parseTaskCard(createTaskCard({
    name: '鐧藉悕鍗曢€氳繃',
    goal: 'validate allow-list',
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

test('validateTaskFiles 鍛戒腑绂佸尯鎴栬秺鐣屾椂 FAIL', () => {
  const task = parseTaskCard(createTaskCard({
    name: '杈圭晫澶辫触',
    goal: '楠岃瘉绂佸尯',
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

test('new_parallel_task CLI 鐢熸垚鍖呭惈鏍稿績瀛楁鐨勪换鍔″崱', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-card-'));
  const dest = path.join(tmpDir, 'task-card.md');
  const result = spawnSync(
    process.execPath,
    [
      'scripts/ops/new_parallel_task.mjs',
      '--name',
      '绀轰緥鍗＄墖',
      '--goal',
      '钀藉湴浠诲姟鍗¤剼鎵嬫灦',
      '--background',
      '鐢ㄤ簬鍞悗澶?AI 骞惰',
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
      cwd: repoRoot,
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 0, `CLI exited with ${result.stderr || result.stdout}`);
  const contents = fs.readFileSync(dest, 'utf8');
  assert.ok(contents.includes('name: 绀轰緥鍗＄墖'));
  assert.ok(contents.includes('goal: 钀藉湴浠诲姟鍗¤剼鎵嬫灦'));
  assert.ok(contents.includes('allowed_files:'));
  assert.ok(contents.includes('## Test Commands'));
});

test('validate_task_boundary CLI 鏀寔鏄惧紡鏂囦欢鍒楄〃', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-boundary-'));
  const taskPath = path.join(tmpDir, 'task-card.md');
  fs.writeFileSync(taskPath, createTaskCard({
    name: '鏄惧紡鏂囦欢杈圭晫鏍￠獙',
    goal: '楠岃瘉 CLI',
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
      cwd: repoRoot,
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
      cwd: repoRoot,
      encoding: 'utf8',
    },
  );
  assert.equal(failResult.status, 1);
  assert.ok(failResult.stdout.includes('FAIL'));
});

test('readGitChangedFiles 鏀寔 staged / working / all 涓夌妯″紡', () => {
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

test('task card supports window/slot/lock metadata fields', () => {
  const card = createTaskCard({
    name: 'window-meta',
    goal: 'validate metadata parsing',
    owner: 'W10',
    branch: 'codex/window-meta',
    worktree: '.worktrees/window-meta',
    window: 'W10',
    domain: 'system-governance',
    slot: 'T1',
    sharedLockTargets: ['openapi/src/openapi.yaml', 'scripts/openapi/**'],
  });

  const parsed = parseTaskCard(card);
  assert.equal(parsed.window, 'W10');
  assert.equal(parsed.domain, 'system-governance');
  assert.equal(parsed.slot, 'T1');
  assert.deepEqual(parsed.sharedLockTargets, ['openapi/src/openapi.yaml', 'scripts/openapi/**']);
});

test('validate_task_boundary fails when shared lock target changed without active lock', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-shared-lock-fail-'));
  const taskPath = path.join(tmpDir, 'task-card.md');
  const policyPath = path.join(tmpDir, 'policy.json');
  const registryPath = path.join(tmpDir, 'registry.json');

  fs.writeFileSync(taskPath, createTaskCard({
    name: 'shared-lock-fail',
    goal: 'must fail without lock',
    owner: 'W10',
    branch: 'codex/shared-lock-fail',
    worktree: '.worktrees/shared-lock-fail',
    window: 'W10',
    domain: 'system-governance',
    slot: 'T1',
    allowedFiles: ['openapi/src/**'],
    sharedLockTargets: ['openapi/src/openapi.yaml'],
  }), 'utf8');

  fs.writeFileSync(policyPath, JSON.stringify({
    version: 'v2',
    locked_patterns: ['openapi/src/openapi.yaml'],
  }, null, 2), 'utf8');
  fs.writeFileSync(registryPath, JSON.stringify({
    version: 'v2',
    locks: [],
  }, null, 2), 'utf8');

  const result = spawnSync(
    process.execPath,
    [
      'scripts/ops/validate_task_boundary.mjs',
      '--task',
      taskPath,
      '--file',
      'openapi/src/openapi.yaml',
      '--policy-file',
      policyPath,
      '--lock-registry',
      registryPath,
    ],
    {
      cwd: repoRoot,
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 1);
  assert.ok(result.stdout.includes('shared lock'));
});

test('validate_task_boundary passes when shared lock is active for current task', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-shared-lock-pass-'));
  const taskPath = path.join(tmpDir, 'task-card.md');
  const policyPath = path.join(tmpDir, 'policy.json');
  const registryPath = path.join(tmpDir, 'registry.json');

  fs.writeFileSync(taskPath, createTaskCard({
    name: 'shared-lock-pass',
    goal: 'must pass with lock',
    owner: 'W10',
    branch: 'codex/shared-lock-pass',
    worktree: '.worktrees/shared-lock-pass',
    window: 'W10',
    domain: 'system-governance',
    slot: 'T1',
    allowedFiles: ['openapi/src/**'],
    sharedLockTargets: ['openapi/src/openapi.yaml'],
  }), 'utf8');

  fs.writeFileSync(policyPath, JSON.stringify({
    version: 'v2',
    locked_patterns: ['openapi/src/openapi.yaml'],
  }, null, 2), 'utf8');
  fs.writeFileSync(registryPath, JSON.stringify({
    version: 'v2',
    locks: [
      {
        pattern: 'openapi/src/openapi.yaml',
        task: path.resolve(taskPath).replace(/\\/g, '/'),
        owner: 'W10',
        acquired_at: '2026-04-14T16:00:00+08:00',
      },
    ],
  }, null, 2), 'utf8');

  const result = spawnSync(
    process.execPath,
    [
      'scripts/ops/validate_task_boundary.mjs',
      '--task',
      taskPath,
      '--file',
      'openapi/src/openapi.yaml',
      '--policy-file',
      policyPath,
      '--lock-registry',
      registryPath,
    ],
    {
      cwd: repoRoot,
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.ok(result.stdout.includes('PASS'));
});

