import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const currentFilePath = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(currentFilePath), '..', '..');

function run(args, cwd = repoRoot) {
  return spawnSync(process.execPath, args, {
    cwd,
    encoding: 'utf8',
  });
}

test('register_shared_lock acquire and release lifecycle', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'shared-lock-lifecycle-'));
  const policyPath = path.join(tmpDir, 'policy.json');
  const registryPath = path.join(tmpDir, 'registry.json');
  const taskPath = path.join(tmpDir, 'task.md');

  fs.writeFileSync(policyPath, JSON.stringify({
    version: 'v2',
    locked_patterns: ['openapi/src/openapi.yaml'],
  }, null, 2));
  fs.writeFileSync(registryPath, JSON.stringify({
    version: 'v2',
    locks: [],
  }, null, 2));
  fs.writeFileSync(taskPath, '---\nname: demo\nowner: W10\n---\n');

  const acquire = run([
    'scripts/ops/register_shared_lock.mjs',
    '--mode', 'acquire',
    '--task', taskPath,
    '--owner', 'W10',
    '--pattern', 'openapi/src/openapi.yaml',
    '--policy-file', policyPath,
    '--lock-registry', registryPath,
  ]);
  assert.equal(acquire.status, 0, acquire.stderr || acquire.stdout);

  const status = run([
    'scripts/ops/register_shared_lock.mjs',
    '--mode', 'status',
    '--policy-file', policyPath,
    '--lock-registry', registryPath,
  ]);
  assert.equal(status.status, 0, status.stderr || status.stdout);
  assert.ok(status.stdout.includes('openapi/src/openapi.yaml'));

  const release = run([
    'scripts/ops/register_shared_lock.mjs',
    '--mode', 'release',
    '--task', taskPath,
    '--owner', 'W10',
    '--policy-file', policyPath,
    '--lock-registry', registryPath,
  ]);
  assert.equal(release.status, 0, release.stderr || release.stdout);

  const finalState = JSON.parse(fs.readFileSync(registryPath, 'utf8'));
  assert.deepEqual(finalState.locks, []);
});

test('register_shared_lock blocks conflicting owner', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'shared-lock-conflict-'));
  const policyPath = path.join(tmpDir, 'policy.json');
  const registryPath = path.join(tmpDir, 'registry.json');
  const taskA = path.join(tmpDir, 'task-a.md');
  const taskB = path.join(tmpDir, 'task-b.md');

  fs.writeFileSync(policyPath, JSON.stringify({
    version: 'v2',
    locked_patterns: ['openapi/src/openapi.yaml'],
  }, null, 2));
  fs.writeFileSync(registryPath, JSON.stringify({
    version: 'v2',
    locks: [],
  }, null, 2));
  fs.writeFileSync(taskA, '---\nname: taskA\nowner: W10\n---\n');
  fs.writeFileSync(taskB, '---\nname: taskB\nowner: W9\n---\n');

  const firstAcquire = run([
    'scripts/ops/register_shared_lock.mjs',
    '--mode', 'acquire',
    '--task', taskA,
    '--owner', 'W10',
    '--pattern', 'openapi/src/openapi.yaml',
    '--policy-file', policyPath,
    '--lock-registry', registryPath,
  ]);
  assert.equal(firstAcquire.status, 0, firstAcquire.stderr || firstAcquire.stdout);

  const conflictAcquire = run([
    'scripts/ops/register_shared_lock.mjs',
    '--mode', 'acquire',
    '--task', taskB,
    '--owner', 'W9',
    '--pattern', 'openapi/src/openapi.yaml',
    '--policy-file', policyPath,
    '--lock-registry', registryPath,
  ]);
  assert.equal(conflictAcquire.status, 1);
  assert.ok((conflictAcquire.stderr + conflictAcquire.stdout).includes('already locked'));
});
