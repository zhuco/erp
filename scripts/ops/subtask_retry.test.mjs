import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isTransientFailure, runWithRetry } from './lib/subtask_retry.mjs';

test('isTransientFailure identifies gateway failures', () => {
  assert.equal(isTransientFailure('unexpected status 502 Bad Gateway'), true);
  assert.equal(isTransientFailure('Upstream request failed with 504'), true);
  assert.equal(isTransientFailure('validation failed: bad input'), false);
});

test('runWithRetry succeeds without retry when first attempt passes', async () => {
  const calls = [];
  const result = await runWithRetry({
    maxRetries: 3,
    execute: async (attempt) => {
      calls.push(attempt);
      return { code: 0, stdout: 'ok', stderr: '' };
    },
  });

  assert.equal(result.ok, true);
  assert.equal(result.attempts, 1);
  assert.equal(result.retried, false);
  assert.deepEqual(calls, [1]);
});

test('runWithRetry retries transient failure and then succeeds', async () => {
  const calls = [];
  const result = await runWithRetry({
    maxRetries: 2,
    delayMs: 0,
    execute: async (attempt) => {
      calls.push(attempt);
      if (attempt === 1) {
        return { code: 1, stdout: '', stderr: '502 Bad Gateway' };
      }
      return { code: 0, stdout: 'ok', stderr: '' };
    },
  });

  assert.equal(result.ok, true);
  assert.equal(result.attempts, 2);
  assert.equal(result.retried, true);
  assert.deepEqual(calls, [1, 2]);
});

test('runWithRetry does not retry non-transient failure', async () => {
  const calls = [];
  const result = await runWithRetry({
    maxRetries: 5,
    execute: async (attempt) => {
      calls.push(attempt);
      return { code: 2, stdout: '', stderr: 'schema validation failed' };
    },
  });

  assert.equal(result.ok, false);
  assert.equal(result.attempts, 1);
  assert.equal(result.retried, false);
  assert.equal(result.transient, false);
  assert.deepEqual(calls, [1]);
});

test('runWithRetry stops after max retries for transient failures', async () => {
  const calls = [];
  const result = await runWithRetry({
    maxRetries: 2,
    delayMs: 0,
    execute: async (attempt) => {
      calls.push(attempt);
      return { code: 1, stdout: '', stderr: '502 Bad Gateway' };
    },
  });

  assert.equal(result.ok, false);
  assert.equal(result.attempts, 3);
  assert.equal(result.retried, true);
  assert.equal(result.transient, true);
  assert.deepEqual(calls, [1, 2, 3]);
});
