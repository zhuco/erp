import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig, devices } from '@playwright/test';

const E2E_HOST = process.env.PW_E2E_HOST ?? '127.0.0.1';
const E2E_PORT = Number(process.env.PW_E2E_PORT ?? 52941);
const E2E_BASE_URL = `http://${E2E_HOST}:${E2E_PORT}`;
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const WORKSPACE_ROOT = path.resolve(__dirname, '../..');
const PLAYWRIGHT_BUILD_DIR = path.resolve(WORKSPACE_ROOT, 'build/playwright');

export default defineConfig({
  testDir: './e2e',
  outputDir: path.join(PLAYWRIGHT_BUILD_DIR, 'test-results'),
  reporter: [
    ['list'],
    ['html', { outputFolder: path.join(PLAYWRIGHT_BUILD_DIR, 'html-report'), open: 'never' }],
  ],
  timeout: 30_000,
  expect: {
    timeout: 10_000
  },
  use: {
    baseURL: E2E_BASE_URL,
    trace: 'on-first-retry',
    contextOptions: {
      reducedMotion: 'reduce'
    }
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chromium'] }
    }
  ],
  webServer: {
    command: `node ./node_modules/vite/bin/vite.js --host ${E2E_HOST} --port ${E2E_PORT} --strictPort`,
    port: E2E_PORT,
    reuseExistingServer: false,
    timeout: 120_000,
    env: {
      VITE_USE_MOCKS: 'true',
      VITE_API_BASE_URL: 'http://127.0.0.1:8080'
    }
  }
});
