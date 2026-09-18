import fs from 'node:fs';
import { validatePerf02, validateWebPerf02 } from './perf02-validation.mjs';

const file = process.argv[2];
if (!file) throw new Error('Usage: node qa/performance/validate-report.mjs <report.json>');
const value = JSON.parse(fs.readFileSync(file, 'utf8'));
const result = value.result && value.environment
  ? validateWebPerf02(value)
  : validatePerf02(value);
console.log(`PERF-02 report valid: P95 ${result.p95} ms; rounds ${result.roundCounts.join('/')}`);
