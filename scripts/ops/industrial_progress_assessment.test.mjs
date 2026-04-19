import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';

test('industrial progress assessment emits expected JSON shape', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'industrial-progress-'));
  const outputPath = path.join(tempRoot, 'nested', 'industrial-progress.json');

  const result = spawnSync(
    process.execPath,
    ['scripts/assess_industrial_progress.mjs', '--out', outputPath],
    {
      cwd: 'F:/erp',
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.equal(fs.existsSync(outputPath), true, 'expected output file to be created');

  const report = JSON.parse(fs.readFileSync(outputPath, 'utf8'));
  assert.equal(typeof report.generated_at, 'string');
  assert.equal(typeof report.repo_root, 'string');
  assert.equal(typeof report.metrics, 'object');
  assert.equal(typeof report.gates, 'object');
  assert.equal(typeof report.placeholder_route_references, 'object');

  assert.equal(typeof report.metrics.web_pages_tsx, 'number');
  assert.equal(typeof report.metrics.web_api_modules_ts, 'number');
  assert.equal(typeof report.metrics.erp_api_tests_rs, 'number');
  assert.equal(typeof report.metrics.openapi_v1_paths, 'number');

  assert.equal(typeof report.gates.repo_verify, 'object');
  assert.equal(typeof report.gates.accept_web, 'object');
  assert.equal(typeof report.gates.backend_release_gate_precheck, 'object');

  assert.equal(typeof report.gates.repo_verify.status, 'string');
  assert.equal(typeof report.gates.accept_web.status, 'string');
  assert.equal(typeof report.gates.backend_release_gate_precheck.status, 'string');

  assert.equal(typeof report.placeholder_route_references.import_count, 'number');
  assert.equal(typeof report.placeholder_route_references.usage_count, 'number');
  assert.equal(typeof report.placeholder_route_references.total_mentions, 'number');
});
