import assert from 'node:assert/strict';

const finiteNonNegative = (value, name) => {
  assert.equal(Number.isFinite(value) && value >= 0, true, `${name} must be finite and non-negative`);
};

export function validatePerf02(report) {
  assert.ok(report && typeof report === 'object', 'PERF-02 report is missing');
  assert.equal(report.scenario, 'PERF-02');
  assert.equal(report.build, 'release');
  assert.equal(report.rows, 10_000);
  assert.equal(report.columns, 100);
  assert.equal(report.left_pinned, 2);
  assert.equal(report.right_pinned, 2);
  assert.equal(report.warmup_frames, 5);
  assert.equal(report.rounds, 3);
  assert.equal(report.seconds_per_round, 10);
  assert.equal(report.rounds_complete, true);
  assert.deepEqual(report.viewport_points, [1280, 800]);
  assert.ok(Number.isFinite(report.pixels_per_point) && report.pixels_per_point > 0);
  if (Object.hasOwn(report, 'presentation_mode')) {
    assert.equal(['default', 'auto_no_vsync'].includes(report.presentation_mode), true,
      'presentation_mode is invalid');
  }
  finiteNonNegative(report.measured_seconds, 'measured_seconds');
  assert.ok(report.measured_seconds >= 30, 'PERF-02 must measure at least 30 seconds');

  assert.ok(Array.isArray(report.samples) && report.samples.length > 0, 'PERF-02 samples are missing');
  const roundCounts = [0, 0, 0];
  let previousElapsed = -1;
  const intervals = [];
  for (const [index, sample] of report.samples.entries()) {
    for (const field of ['elapsed_ms', 'frame_interval_ms', 'draw_cpu_ms', 'cell_text_calls']) {
      finiteNonNegative(sample[field], `samples[${index}].${field}`);
    }
    if (Object.hasOwn(sample, 'previous_frame_cpu_ms') && sample.previous_frame_cpu_ms !== null) {
      finiteNonNegative(sample.previous_frame_cpu_ms, `samples[${index}].previous_frame_cpu_ms`);
    }
    for (const field of ['viewport_focused', 'viewport_minimized', 'viewport_occluded']) {
      if (Object.hasOwn(sample, field)) {
        assert.equal(sample[field] === null || typeof sample[field] === 'boolean', true,
          `samples[${index}].${field} must be boolean or null`);
      }
    }
    assert.equal(Number.isInteger(sample.round) && sample.round >= 1 && sample.round <= 3, true,
      `samples[${index}].round must be 1..3`);
    assert.equal(sample.focused, true, `samples[${index}] was not focused`);
    assert.ok(sample.elapsed_ms >= previousElapsed, `samples[${index}] is out of time order`);
    assert.ok(sample.elapsed_ms < 30_000, `samples[${index}] falls outside the measured window`);
    assert.equal(sample.round, Math.floor(sample.elapsed_ms / 10_000) + 1,
      `samples[${index}] has an incorrect round`);
    previousElapsed = sample.elapsed_ms;
    roundCounts[sample.round - 1] += 1;
    intervals.push(sample.frame_interval_ms);
  }
  assert.ok(roundCounts.every(count => count >= 30), `each round needs at least 30 samples: ${roundCounts}`);

  intervals.sort((left, right) => left - right);
  const p95Index = Math.ceil(intervals.length * 0.95) - 1;
  const p95 = intervals[Math.max(0, p95Index)];
  finiteNonNegative(report.frame_interval_p95_ms, 'frame_interval_p95_ms');
  assert.ok(Math.abs(report.frame_interval_p95_ms - p95) <= 1e-9,
    `reported P95 ${report.frame_interval_p95_ms} does not match samples ${p95}`);
  assert.ok(p95 <= 32, `frame interval P95 ${p95} ms exceeds 32 ms`);
  return {p95, roundCounts};
}

export function validateWebPerf02(payload) {
  assert.ok(payload?.rawResult, 'Raw Web PERF-02 result is missing');
  assert.deepEqual(JSON.parse(payload.rawResult), payload.result, 'Parsed result differs from raw Web report');
  assert.deepEqual(payload.environment?.viewport, {width: 1280, height: 800});
  assert.ok(payload.environment?.browserVersion, 'Browser version is missing');
  assert.ok(payload.environment?.userAgent, 'Browser user agent is missing');
  assert.equal(Number.isFinite(payload.environment?.devicePixelRatio)
    && payload.environment.devicePixelRatio > 0, true, 'Device pixel ratio is missing');
  assert.equal(Number.isInteger(payload.environment?.hardwareConcurrency)
    && payload.environment.hardwareConcurrency > 0, true, 'CPU thread count is missing');
  return validatePerf02(payload.result);
}
