// PERF-02 collector for the Rust/WASM grid's own timed browser measurement.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { validateWebPerf02 } from './perf02-validation.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const output = path.join(root, 'qa/performance');
const require = createRequire(import.meta.url);
const cliPackage = require.resolve('@playwright/cli/package.json');
const cli = path.join(path.dirname(cliPackage), 'playwright-cli.js');
const session = 'erp-grid-perf-02';
const url = 'http://127.0.0.1:15440/?grid-perf';
const lockPath = path.join(output, 'run-web.lock');
const resultPath = path.join(output, 'web-latest.json');
const captureSeconds = [8, 17, 26];
const screenshots = captureSeconds.map(
  second => path.join(output, `web-${second}s.png`),
);

fs.mkdirSync(output, { recursive: true });
const lock = fs.openSync(lockPath, 'wx');
fs.writeFileSync(lock, String(process.pid));
let opened = false;

function command(...args) {
  const text = execFileSync(process.execPath, [cli, `-s=${session}`, ...args], {
    cwd: root,
    encoding: 'utf8',
    timeout: 60000,
    windowsHide: true,
    maxBuffer: 8 * 1024 * 1024,
  });
  if (/^### Error/m.test(text)) throw new Error(text);
  return text;
}

function decodePayload(text) {
  const match = text.match(/ERP_GRID_PERF_B64:([A-Za-z0-9+/=]+)/);
  if (!match) throw new Error(`Playwright did not return the PERF-02 payload:\n${text}`);
  return JSON.parse(Buffer.from(match[1], 'base64').toString('utf8'));
}

try {
  try { command('close'); } catch { /* Remove only a stale session with this fixed name. */ }
  command('open', 'about:blank', '--browser', process.env.ERP_PERF_BROWSER || 'msedge');
  opened = true;
  command('resize', '1280', '800');

  const browserCode = `async page => {
    const url = ${JSON.stringify(url)};
    const screenshotPaths = ${JSON.stringify(screenshots)};
    const captureSeconds = ${JSON.stringify(captureSeconds)};
    const businessRequests = [];
    page.on('request', request => {
      const requested = request.url();
      if (/127\\.0\\.0\\.1:15441|\\/(?:auth|v1)\\//.test(requested)) businessRequests.push(requested);
    });
    await page.goto(url, {waitUntil: 'domcontentloaded'});
    await page.waitForFunction(() => document.querySelector('canvas'));
    const captures = screenshotPaths.map(async (target, index) => {
      await page.waitForTimeout(captureSeconds[index] * 1000);
      await page.screenshot({path: target});
    });
    await page.waitForFunction(
      () => document.body?.getAttribute('data-grid-perf-result'),
      null,
      {timeout: 45000},
    );
    await Promise.all(captures);
    const rawResult = await page.locator('body').getAttribute('data-grid-perf-result');
    if (!rawResult) throw new Error('Rust PERF-02 result attribute is missing');
    let result;
    try { result = JSON.parse(rawResult); } catch (error) {
      throw new Error('Rust PERF-02 result is not valid JSON: ' + error.message);
    }
    const browser = page.context().browser();
    const environment = await page.evaluate(() => ({
      userAgent: navigator.userAgent,
      devicePixelRatio: window.devicePixelRatio,
      hardwareConcurrency: navigator.hardwareConcurrency,
      viewport: {width: window.innerWidth, height: window.innerHeight},
    }));
    environment.browserVersion = browser.version();
    let processInfo;
    try {
      const cdp = await browser.newBrowserCDPSession();
      const response = await cdp.send('SystemInfo.getProcessInfo');
      processInfo = {available: true, processes: response.processInfo};
      await cdp.detach();
    } catch (error) {
      processInfo = {available: false, error: String(error)};
    }
    const payload = {
      collectedAt: new Date().toISOString(),
      url,
      environment,
      rawResult,
      result,
      processInfo,
      businessRequests,
      boundaries: [
        'Rust page owns the 30 second measurement and 5 warm-up frames',
        'CDP process IDs support external Windows RSS sampling only',
        'No JavaScript heap value is reported as total browser memory',
      ],
    };
    const encoded = await page.evaluate(value => {
      const bytes = new TextEncoder().encode(value);
      let binary = '';
      for (const byte of bytes) binary += String.fromCharCode(byte);
      return btoa(binary);
    }, JSON.stringify(payload));
    return 'ERP_GRID_PERF_B64:' + encoded;
  }`;
  const payload = decodePayload(command('run-code', browserCode));
  const ids = (payload.processInfo?.processes ?? []).map(process => process.id).filter(id => Number.isSafeInteger(id) && id > 0);
  if (process.platform === 'win32' && ids.length) {
    try {
      payload.processMemory = {available: true, values: JSON.parse(execFileSync('pwsh', ['-NoProfile', '-Command', `Get-Process -Id @(${ids.join(',')}) -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,WorkingSet64,PeakWorkingSet64 | ConvertTo-Json -Compress`], {encoding:'utf8', windowsHide:true, timeout:10000}))};
    } catch (error) { payload.processMemory = {available:false, error:String(error)}; }
  } else { payload.processMemory = {available:false, reason:'Windows process IDs unavailable'}; }
  payload.boundaries.push('Per-process peak working sets are lifetime high-water marks; their sum is not simultaneous peak memory');
  fs.writeFileSync(resultPath, JSON.stringify(payload, null, 2) + '\n');
  validateWebPerf02(payload);
  if (!payload.environment.browserVersion || !payload.environment.userAgent) {
    throw new Error('Browser version or user agent is missing');
  }
  if (!(payload.environment.devicePixelRatio > 0)) throw new Error('Device pixel ratio is missing');
  if (!(payload.environment.hardwareConcurrency > 0)) throw new Error('CPU thread count is missing');
  if (payload.businessRequests.length) {
    throw new Error(`PERF-02 unexpectedly called business APIs: ${payload.businessRequests.join(', ')}`);
  }
  for (const screenshot of screenshots) {
    if (!fs.existsSync(screenshot) || fs.statSync(screenshot).size === 0) {
      throw new Error(`PERF-02 screenshot is missing: ${screenshot}`);
    }
  }
  fs.writeFileSync(resultPath, JSON.stringify(payload, null, 2) + '\n');
  console.log(`PERF-02 collected: ${path.relative(root, resultPath)}`);
} finally {
  if (opened) {
    try { command('close'); } catch { console.error(`Could not confirm closure of browser session ${session}.`); }
  }
  fs.closeSync(lock);
  fs.unlinkSync(lockPath);
}
