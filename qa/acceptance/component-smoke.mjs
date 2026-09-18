// Browser-only component gate. No API, credentials or business writes.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { spawn, execFileSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(import.meta.url);
const cli = path.join(path.dirname(require.resolve('@playwright/cli/package.json')), 'playwright-cli.js');
const output = path.join(root, 'output/playwright/component-smoke');
fs.mkdirSync(output, { recursive: true });
const session = 'erp-component-smoke';
const existing = process.argv.includes('--existing-server');
assert(process.argv.slice(2).every(arg => arg === '--existing-server'), 'Unknown option');
let server;
let opened = false;
const lockPath = path.join(output, 'run.lock');
const lock = fs.openSync(lockPath, 'wx');
fs.writeFileSync(lock, String(process.pid));
function command(...args) {
  const result = execFileSync(process.execPath, [cli, `-s=${session}`, ...args], {
    cwd: root, encoding: 'utf8', timeout: 45000, windowsHide: true, maxBuffer: 4 * 1024 * 1024,
  });
  assert(!/^### Error/m.test(result), result);
  return result;
}
try {
  if (!existing) {
    server = spawn(process.execPath, ['qa/acceptance/serve-web.mjs'], {
      cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'],
    });
    await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Static server startup timed out')), 10000);
      const done = (error) => { clearTimeout(timeout); error ? reject(error) : resolve(); };
      server.once('error', done);
      server.once('exit', code => done(new Error(`Static server exited: ${code}`)));
      server.stderr.on('data', data => fs.appendFileSync(path.join(output, 'server.log'), data));
      server.stdout.on('data', data => {
        fs.appendFileSync(path.join(output, 'server.log'), data);
        if (data.toString().includes('ERP WASM acceptance UI:')) done();
      });
    });
  }
  opened = true;
  command('open', 'about:blank', '--browser', process.env.ERP_SMOKE_BROWSER || 'msedge');
  const log = command('run-code', `async page => {
    await page.setViewportSize({width:1280,height:800});
    const errors=[]; page.on('pageerror',error=>errors.push(error.message));
    await page.goto('http://127.0.0.1:15440/?grid-first-load');
    await page.waitForFunction(()=>document.body.hasAttribute('data-grid-first-load-result'), {}, {timeout:30000});
    const report=JSON.parse(await page.locator('body').getAttribute('data-grid-first-load-result'));
    return {report,errors};
  }`);
  fs.writeFileSync(path.join(output, 'browser.log'), log);
  const line = log.split(/\r?\n/).find(line => line.startsWith('{'));
  assert(line, 'Browser returned no report');
  const { report, errors } = JSON.parse(line);
  assert.deepEqual(errors, [], 'Browser runtime errors');
  assert.equal(report.scenario, 'PERF-01');
  assert.equal(report.error, null);
  assert.equal(report.release, true, 'Build release WASM before the gate');
  assert.equal(report.rows, 10000);
  assert.equal(report.columns, 30);
  assert.equal(report.samples_ms.length, 30, 'All 30 keyboard transitions must complete');
  assert(report.samples_ms.every(value => Number.isFinite(value) && value >= 0));
  // Shared CI runners are not the reference performance machine. Keep raw timing
  // without declaring its 500ms budget passed or changing the reference budget.
  fs.writeFileSync(path.join(output, 'result.json'), JSON.stringify({ report, errors, timing_gate: false }, null, 2));
  console.log('WASM DataGrid: 30 rendered keyboard transitions passed');
} finally {
  try { if (opened) command('close'); }
  finally {
    if (server && server.exitCode === null) server.kill();
    fs.closeSync(lock);
    fs.unlinkSync(lockPath);
  }
}
