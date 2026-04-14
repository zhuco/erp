import { expect, test as base } from '@playwright/test';
import type { Browser, Locator, Page } from '@playwright/test';

// Mock tenant ID (matches tenantOptions.ts)
export const MOCK_TENANT_ID = '11111111-1111-1111-1111-111111111111';
export const MOCK_TOKEN = 'mock-test-token';
const DISABLE_ANIMATIONS_STYLE = `
*,
*::before,
*::after {
  transition-duration: 0s !important;
  animation-duration: 0s !important;
  animation-delay: 0s !important;
}
`;

const PRIMARY_TENANT_LABEL = '主租户';
const escapeRegExp = (value: string): string =>
  value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const tenantLabelRegex = new RegExp(
  `${escapeRegExp(PRIMARY_TENANT_LABEL)}|${escapeRegExp(MOCK_TENANT_ID)}`
);
const WINDOWS_BROWSER_CLOSE_TIMEOUT_MS = 5_000;

type BrowserWithConnection = Browser & {
  _connection?: {
    close: (cause?: string) => void;
  };
};

async function closeBrowserWithWindowsFallback(browser: Browser) {
  const closePromise = browser.close({ reason: 'Test ended.' });

  if (process.platform !== 'win32') {
    await closePromise;
    return;
  }

  let timer: NodeJS.Timeout | undefined;
  const closeState = await Promise.race([
    closePromise.then(() => 'closed' as const),
    new Promise<'timeout'>((resolve) => {
      timer = setTimeout(() => resolve('timeout'), WINDOWS_BROWSER_CLOSE_TIMEOUT_MS);
    })
  ]);

  if (timer) {
    clearTimeout(timer);
  }

  if (closeState === 'timeout') {
    console.warn(
      `[E2E] browser.close() 在 ${WINDOWS_BROWSER_CLOSE_TIMEOUT_MS}ms 内未完成，已强制关闭 Playwright 连接以避免 worker 挂起`
    );
    (browser as BrowserWithConnection)._connection?.close(
      'Windows 下 browser.close() 未返回，强制关闭连接'
    );
  }

  await closePromise;
}

async function clickOption(option: Locator) {
  await option.scrollIntoViewIfNeeded();
  try {
    await option.click();
  } catch {
    await option.click({ force: true });
  }
}

async function waitForVisibleTarget<TTarget extends string>(
  page: Page,
  candidates: Array<{ key: TTarget; locator: Locator }>,
  timeoutMs: number,
): Promise<TTarget | null> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    for (const candidate of candidates) {
      if ((await candidate.locator.count()) === 0) {
        continue;
      }
      if (await candidate.locator.first().isVisible()) {
        return candidate.key;
      }
    }
    await page.waitForTimeout(100);
  }

  return null;
}

// Helper to login and select tenant
export async function loginAndSelectTenant(page: Page) {
  await page.waitForLoadState('domcontentloaded');
  const loginInput = page.getByTestId('login-username-input');
  const shell = page.locator('.workbench-shell');
  const initialVisibleTarget = await waitForVisibleTarget(
    page,
    [
      { key: 'login', locator: loginInput },
      { key: 'shell', locator: shell },
    ],
    5000,
  );

  if (initialVisibleTarget === 'shell') {
    await expect(page.locator('.workbench-shell')).toBeVisible();
    return;
  }

  if (initialVisibleTarget !== 'login') {
    await expect(loginInput).toBeVisible();
  }

  // Fill login form
  const tenantInput = page.getByTestId('login-tenant-input');
  if (await tenantInput.count()) {
    await tenantInput.fill(MOCK_TENANT_ID);
  }
  await loginInput.fill('erp-operator');
  await page.getByTestId('login-password-input').fill('mock-password');
  await page.getByTestId('login-submit').click();

  const tenantSelect = page.getByTestId('tenant-select');
  const postLoginVisibleTarget = await waitForVisibleTarget(
    page,
    [
      { key: 'tenant-select', locator: tenantSelect },
      { key: 'shell', locator: shell },
    ],
    5000,
  );

  if (postLoginVisibleTarget === 'shell') {
    await expect(page.locator('.workbench-shell')).toBeVisible();
    return;
  }

  if (postLoginVisibleTarget !== 'tenant-select') {
    await expect(tenantSelect).toBeVisible();
  }

  await tenantSelect.click();

  const dropdown = page.locator('.ant-select-dropdown:visible').first();
  await expect(dropdown).toBeVisible();
  const listbox = dropdown
    .locator('[role="listbox"]:visible, .rc-virtual-list-holder-inner:visible')
    .first();
  const optionScope = (await listbox.count()) > 0 ? listbox : dropdown;
  await expect(
    optionScope.locator('[role="option"]:visible, .ant-select-item-option:visible').first()
  ).toBeVisible();

  const optionCandidates = [
    optionScope.locator(
      `.ant-select-item-option[data-value="${MOCK_TENANT_ID}"]:visible,` +
        `[role="option"][data-value="${MOCK_TENANT_ID}"]:visible`
    ),
    optionScope.locator('.ant-select-item-option:visible', {
      hasText: PRIMARY_TENANT_LABEL
    }),
    optionScope.locator(
      `[role="option"][aria-label*="${PRIMARY_TENANT_LABEL}"]:visible,` +
        `[role="option"][aria-label*="${MOCK_TENANT_ID}"]:visible,` +
        `.ant-select-item-option[aria-label*="${PRIMARY_TENANT_LABEL}"]:visible,` +
        `.ant-select-item-option[aria-label*="${MOCK_TENANT_ID}"]:visible`
    ),
    optionScope.getByRole('option', { name: PRIMARY_TENANT_LABEL, exact: true }),
    optionScope.getByRole('option', { name: tenantLabelRegex })
  ];

  let optionToClick = optionScope
    .locator('[role="option"], .ant-select-item-option')
    .first();
  for (const candidate of optionCandidates) {
    if ((await candidate.count()) > 0) {
      optionToClick = candidate.first();
      break;
    }
  }

  await expect(optionToClick).toBeVisible();
  await clickOption(optionToClick);

  await page.getByTestId('tenant-submit').click();
  await page.waitForURL((url) => !url.pathname.includes('/login'));
  await expect(page.locator('.workbench-shell')).toBeVisible();
  await expect(page.locator('.ant-layout-sider')).toBeVisible();
}

// Helper to wait for API requests
export async function waitForApiRequest(
  page: Page,
  urlPattern: string | RegExp
): Promise<void> {
  await page.waitForRequest((request) => {
    const url = request.url();
    if (typeof urlPattern === 'string') {
      return url.includes(urlPattern);
    }
    return urlPattern.test(url);
  });
}

// Fixture to inject PrintBridge mock into window
export const test = base.extend<{
  mockPrintBridge: void;
}>({
  browser: [
    async ({ playwright, browserName, connectOptions }, use) => {
      const browserType = playwright[browserName];
      const browser = connectOptions
        ? await browserType.connect(connectOptions)
        : await browserType.launch();

      await use(browser);
      await closeBrowserWithWindowsFallback(browser);
    },
    { scope: 'worker', timeout: 0 }
  ],
  page: async ({ page }, use) => {
    await page.addInitScript((css) => {
      const style = document.createElement('style');
      style.setAttribute('data-testid', 'e2e-disable-animations');
      style.textContent = css;
      const appendStyle = () => {
        if (document.head) {
          document.head.appendChild(style);
        } else {
          document.documentElement.appendChild(style);
        }
      };
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', appendStyle);
      } else {
        appendStyle();
      }
    }, DISABLE_ANIMATIONS_STYLE);

    await use(page);
  },
  mockPrintBridge: async ({ page }, use) => {
    // Inject mock PrintBridge before each test
    await page.addInitScript(() => {
      (window as any).mockPrintBridge = {
        detect: async () => ({ success: true, data: { version: '1.0.0' } }),
        printShippingLabel: async (payload: any) => {
          console.log('[MockPrintBridge] printShippingLabel called', payload);
          (window as any).lastPrintPayload = payload;
          return { success: true };
        },
        getPrinters: async () => ({ success: true, data: [{ name: 'Mock Printer' }] })
      };
    });

    await use();
  }
});

export { expect };
