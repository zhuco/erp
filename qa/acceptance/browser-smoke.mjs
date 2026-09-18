// Real browser -> Rust canvas -> API -> database smoke, using the existing CLI.
// This deliberately accepts only the isolated fixture, never a business tenant.
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const require = createRequire(import.meta.url);
const cliPackage = require.resolve('@playwright/cli/package.json');
const cli = path.join(path.dirname(cliPackage), 'playwright-cli.js');
const session = 'erp-grid-smoke';
const web = 'http://127.0.0.1:15440';
const api = 'http://127.0.0.1:15441';
const tenant = 'a1100000-0000-4000-8000-000000000001';
const credentials = { tenant_id: tenant, email: 'qa.acceptance@example.invalid', password: 'ErpQa!2026' };
const artifacts = path.join(root, 'output/playwright');
fs.mkdirSync(artifacts, { recursive: true });
const lockPath = path.join(root, 'qa/acceptance/browser-smoke.lock');
const lock = fs.openSync(lockPath, 'wx');
fs.writeFileSync(lock, String(process.pid));
let opened = false;
let token;
let original;
let changed = false;
const marker = `QA grid smoke ${Date.now()}`;

function command(...args) {
  const output = execFileSync(process.execPath, [cli, `-s=${session}`, ...args], {
    cwd: root, encoding: 'utf8', timeout: 45000, windowsHide: true, maxBuffer: 4 * 1024 * 1024,
  });
  if (/^### Error/m.test(output)) throw new Error(output);
  return output;
}
function browser(code) {
  return command('run-code', `async page => {
    const frame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    ${code}
  }`);
}
async function request(endpoint, body) {
  const response = await fetch(api + endpoint, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'Content-Type': 'application/json', ...(token ? {Authorization: `Bearer ${token}`} : {}) },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(10000),
  });
  if (!response.ok) throw new Error(`Isolated API ${endpoint}: HTTP ${response.status}`);
  return response.json();
}
async function product() {
  const result = await request(`/v1/tenants/${tenant}/products/query`, {
    schema: 1, offset: 0, limit: 10, filters: {}, sorts: [], search: 'QA-P-001', expression: null,
  });
  assert.equal(result.total, 1, 'The isolated fixture must contain exactly QA-P-001');
  assert.equal(result.data[0].product_code, 'QA-P-001');
  return result.data[0];
}
async function waitForName(name) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    if ((await product()).name === name) return;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error('Browser edit was not confirmed by API readback');
}
function edit(value) {
  browser(`await page.mouse.dblclick(330,298); await frame();
    await page.keyboard.press('Control+A'); await page.keyboard.insertText(${JSON.stringify(value)}); await frame();`);
}

try {
  token = (await request('/auth/login', credentials)).access_token;
  const identity = await request('/me');
  assert.equal(identity.user.tenant_id, tenant);
  assert.equal(identity.user.email, credentials.email);
  original = await product();
  opened = true;
  command('open', web, '--browser', process.env.ERP_SMOKE_BROWSER || 'msedge');
  command('resize', '1280', '720');
  browser(`await page.bringToFront();
    await page.waitForFunction(() => document.querySelector('canvas')?.width >= 1280);
    await frame(); await page.screenshot({path:'output/playwright/smoke-lab.png'});
    await page.mouse.click(85,11); await frame();
    await page.mouse.click(130,43); await page.keyboard.insertText(${JSON.stringify(api)});
    await page.mouse.click(455,43); await page.keyboard.insertText(${JSON.stringify(tenant)});
    await page.mouse.click(156,66); await page.keyboard.insertText(${JSON.stringify(credentials.email)});
    await page.mouse.click(480,66); await page.keyboard.insertText(${JSON.stringify(credentials.password)});
    const loaded = page.waitForResponse(response => response.url().includes('/products/query') && response.status() === 200);
    await page.mouse.click(709,66); await loaded; await frame();
    const searched = page.waitForResponse(response => response.url().includes('/products/query') && response.request().postData()?.includes('QA-P-001'));
    await page.mouse.click(150,43); await page.keyboard.insertText('QA-P-001');
    await searched;
    await frame();`);
  edit(marker);
  browser(`await page.mouse.click(161,11,{delay:120}); await frame();
    await page.screenshot({path:'output/playwright/smoke-confirmation.png'});`);
  assert.equal((await product()).name, original.name, 'Navigation must not save before confirmation');
  browser(`await page.mouse.click(68,99,{delay:120}); await frame();`);
  changed = true;
  await waitForName(marker);
  browser(`await page.screenshot({path:'output/playwright/smoke-saved.png'});
    await page.mouse.click(85,11); await frame();`);
  edit(original.name);
  browser(`await page.keyboard.press('Enter'); await frame();`);
  await waitForName(original.name);
  changed = false;
  const result = {
    at: new Date().toISOString(), web, api, tenant,
    checks: ['authenticated fixture', 'canvas login and filtered query', 'navigation does not save',
      'save-and-leave API readback', 'restore original via canvas editor'],
    boundaries: ['isolated fixture only', 'not a whole-ERP release or performance test'],
  };
  fs.writeFileSync(path.join(artifacts, 'browser-smoke.json'), JSON.stringify(result, null, 2) + '\n');
  console.log('Rust Web browser smoke passed; original fixture value restored through UI.');
} finally {
  if (changed) console.error(`QA fixture may retain ${marker}; inspect before rerunning. No direct database rollback was attempted.`);
  if (opened) {
    try { command('close'); } catch { console.error(`Could not confirm closure of browser session ${session}.`); }
  }
  fs.closeSync(lock);
  fs.unlinkSync(lockPath);
}
