import { defineConfig, devices } from '@playwright/test';

const E2E_HOST = process.env.PW_E2E_HOST ?? '127.0.0.1';
const E2E_PORT = Number(process.env.PW_E2E_PORT ?? 52941);
const E2E_BASE_URL = `http://${E2E_HOST}:${E2E_PORT}`;

export default defineConfig({
  testDir: './e2e',
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
