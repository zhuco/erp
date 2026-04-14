import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { calculateScorecard, renderMarkdown } from './lib/scorecard.mjs';

const baseInput = () => ({
  date: '2026-04-14',
  scores: {
    parallel_isolation: 4.8,
    code_and_tests: 4.6,
    merge_governance: 4.5,
    observability_and_audit: 4.4,
    delivery_efficiency: 4.3,
    stability: 4.7,
  },
  caps: {
    main_direct_edits: false,
    success_without_persistence: false,
    db_skip_or_pool_timeout: false,
    rollback_plan_missing: false,
    multiple_ai_same_branch: false,
    logic_deleted_to_fix_compile: false,
  },
  recent_p0_regressions: 0,
  gate_effective: true,
  risks: ['平台售后补偿重试仍缺可视化追踪'],
  next_priorities: ['补齐平台联动失败补偿审计'],
  notes: ['今日发生 1 次共享文件锁等待，但未越界'],
});

const closeMatch = (actual, expected, label) => {
  assert.ok(Math.abs(actual - expected) < 1e-6, `${label} expected ${expected}, got ${actual}`);
};

test('加权总分按照权重计算', () => {
  const result = calculateScorecard(baseInput());
  closeMatch(result.finalTotal, 4.545, 'finalTotal');
  assert.equal(result.triggeredRules.length, 0);
});

test('main 直改触发总分封顶', () => {
  const input = baseInput();
  input.caps.main_direct_edits = true;
  const result = calculateScorecard(input);
  assert.equal(result.finalTotal, 4.2);
  assert.ok(result.triggeredRules.some((rule) => rule.key === 'main_direct_edits'));
});

test('单维度封顶与封底生效', () => {
  const input = baseInput();
  input.scores.merge_governance = 4.3;
  input.scores.code_and_tests = 2.6;
  input.caps.rollback_plan_missing = true;
  input.caps.logic_deleted_to_fix_compile = true;
  const result = calculateScorecard(input);
  assert.equal(result.adjustedScores.merge_governance, 3.8);
  assert.equal(result.adjustedScores.code_and_tests, 3.0);
  assert.ok(result.triggeredRules.some((rule) => rule.key === 'rollback_plan_missing'));
  assert.ok(result.triggeredRules.some((rule) => rule.key === 'logic_deleted_to_fix_compile'));
});

test('单维度低于 4.2 则未达标', () => {
  const input = baseInput();
  input.scores.observability_and_audit = 4.1;
  const result = calculateScorecard(input);
  assert.equal(result.isQualified, false);
});

test('recent_p0_regressions 大于 0 则未达标', () => {
  const input = baseInput();
  input.recent_p0_regressions = 2;
  const result = calculateScorecard(input);
  assert.equal(result.isQualified, false);
});

test('gate_effective 为 false 则未达标', () => {
  const input = baseInput();
  input.gate_effective = false;
  const result = calculateScorecard(input);
  assert.equal(result.isQualified, false);
});

test('Markdown 输出包含必要字段', () => {
  const result = calculateScorecard(baseInput());
  const markdown = renderMarkdown(result);
  assert.ok(markdown.includes('日期'));
  assert.ok(markdown.includes('加权总分'));
  assert.ok(markdown.includes('是否达标'));
  assert.ok(markdown.includes('触发封顶/封底规则'));
  assert.ok(markdown.includes('今日风险'));
  assert.ok(markdown.includes('次日重点'));
  assert.ok(markdown.includes('备注'));
});

test('评分卡 CLI 可将 JSON 输入写为 Markdown 文件', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'parallel-scorecard-'));
  const inputPath = path.join(tmpDir, 'input.json');
  const outputPath = path.join(tmpDir, 'scorecard.md');
  fs.writeFileSync(inputPath, JSON.stringify(baseInput(), null, 2), 'utf8');

  const result = spawnSync(
    process.execPath,
    [
      'scripts/ops/generate_parallel_scorecard.mjs',
      '--input',
      inputPath,
      '--output',
      outputPath,
    ],
    {
      cwd: 'F:/erp',
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  const markdown = fs.readFileSync(outputPath, 'utf8');
  assert.ok(markdown.includes('多 AI 并行评分卡'));
  assert.ok(markdown.includes('加权总分'));
  assert.ok(markdown.includes('是否达标'));
});
