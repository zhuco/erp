import test from 'node:test';
import assert from 'node:assert/strict';
import { validatePerf02, validateWebPerf02 } from './perf02-validation.mjs';

const report = {
  scenario: 'PERF-02', build: 'release', rows: 10_000, columns: 100,
  left_pinned: 2, right_pinned: 2, warmup_frames: 5, rounds: 3,
  seconds_per_round: 10, measured_seconds: 30.01, rounds_complete: true,
  viewport_points: [1280, 800], pixels_per_point: 1,
  frame_interval_p95_ms: 20, interval_budget_met: true,
  samples: Array.from({length: 1500}, (_, index) => ({
    elapsed_ms: index * 20, frame_interval_ms: 20, draw_cpu_ms: 2,
    cell_text_calls: 297, focused: true, round: Math.floor(index / 500) + 1,
  })),
};
const baseline = {
  result: report, rawResult: JSON.stringify(report),
  environment: {viewport: {width: 1280, height: 800}, browserVersion: 'fixture',
    userAgent: 'fixture', devicePixelRatio: 1, hardwareConcurrency: 12},
};
const mutate = callback => {
  const value = structuredClone(baseline);
  callback(value);
  value.rawResult = JSON.stringify(value.result);
  return value;
};

test('valid samples satisfy the independent gate', () => {
  const checked = validateWebPerf02(baseline);
  assert.deepEqual(checked.roundCounts, [500, 500, 500]);
  assert.equal(checked.p95, 20);
});

test('rejects a wrong scenario', () => {
  assert.throws(() => validateWebPerf02(mutate(value => { value.result.scenario = 'PERF-01'; })));
});

test('rejects a sample collected while unfocused', () => {
  assert.throws(() => validateWebPerf02(mutate(value => { value.result.samples[20].focused = false; })));
});

test('rejects a forged passing P95', () => {
  assert.throws(() => validateWebPerf02(mutate(value => { value.result.frame_interval_p95_ms = 1; })));
});

test('recomputed P95 rejects slow samples even when the report claims success', () => {
  assert.throws(() => validateWebPerf02(mutate(value => {
    value.result.samples.forEach(sample => { sample.frame_interval_ms = 40; });
    value.result.frame_interval_p95_ms = 40;
    value.result.interval_budget_met = true;
  })));
});

test('core gate accepts the same report in native shape', () => {
  const report = structuredClone(baseline.result);
  report.platform = 'windows';
  assert.equal(validatePerf02(report).p95, 20);
});

test('rejects inconsistent raw and decoded reports', () => {
  const changed = structuredClone(baseline);
  changed.result.rows = 1;
  assert.throws(() => validateWebPerf02(changed), /differs from raw/);
});

test('accepts typed optional native diagnostics and rejects malformed values', () => {
  const valid = mutate(value => {
    value.result.samples.forEach(sample => {
      sample.previous_frame_cpu_ms = 3;
      sample.viewport_focused = true;
      sample.viewport_minimized = false;
      sample.viewport_occluded = null;
    });
  });
  assert.equal(validateWebPerf02(valid).p95, 20);
  assert.throws(() => validateWebPerf02(mutate(value => {
    value.result.samples[0].previous_frame_cpu_ms = -1;
  })));
  assert.throws(() => validateWebPerf02(mutate(value => {
    value.result.samples[0].viewport_occluded = "false";
  })));
});
