import { expect, loginAndSelectTenant, test } from './fixtures';

type PerfSummary = {
  samples: number[];
  avg: number;
  p95: number;
  max: number;
};

const asBudget = (envName: string, fallback: number) => {
  const raw = Number(process.env[envName]);
  return Number.isFinite(raw) && raw > 0 ? raw : fallback;
};

const summarize = (samples: number[]): PerfSummary => {
  const sorted = [...samples].sort((a, b) => a - b);
  const avg = Math.round(samples.reduce((sum, item) => sum + item, 0) / samples.length);
  const p95Index = Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95));
  return {
    samples,
    avg,
    p95: sorted[p95Index] ?? 0,
    max: sorted[sorted.length - 1] ?? 0,
  };
};

test('order review should satisfy industrial interaction budgets', async ({ page }) => {
  const visualBudgetMs = asBudget('PW_PERF_VISUAL_BUDGET_MS', 280);
  const settleBudgetMs = asBudget('PW_PERF_SETTLE_BUDGET_MS', 650);
  const tabBudgetMs = asBudget('PW_PERF_TAB_BUDGET_MS', 300);

  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const leftPanel = page.getByTestId('order-review-left-ag-panel');
  const mainGrid = page.getByTestId('order-review-data-grid');
  await expect(leftPanel).toBeVisible();
  await expect(mainGrid).toBeVisible();

  const cases = [
    { key: 'platform:taobao', label: '淘宝' },
    { key: 'platform:jd', label: '京东' },
    { key: 'platform:pdd', label: '拼多多' },
    { key: 'platform:douyin', label: '抖音' },
  ];

  const visualSamples: number[] = [];
  const settleSamples: number[] = [];

  for (const item of cases) {
    const row = leftPanel.locator(`[row-id="${item.key}"] [role="gridcell"]`).first();
    const clearAllPlatform = leftPanel.locator('[row-id="platform:*"] [role="gridcell"]').first();
    await expect(row).toBeVisible();
    await expect(clearAllPlatform).toBeVisible();

    const start = Date.now();
    await row.click();
    await expect(leftPanel.locator(`[row-id="${item.key}"].order-review-left-ag-row-checked`)).toHaveCount(1);
    const visualElapsed = Date.now() - start;
    visualSamples.push(visualElapsed);

    const platformCells = mainGrid.locator('.ag-center-cols-container .ag-row .ag-cell[col-id="platform"]');
    await expect
      .poll(async () => {
        const values = (await platformCells.allTextContents())
          .map((text) => text.trim())
          .filter(Boolean);
        return values.length > 0 && values.every((text) => text.includes(item.label));
      })
      .toBe(true);
    settleSamples.push(Date.now() - start);

    await clearAllPlatform.click();
    await expect(leftPanel.locator('[row-id="platform:*"].order-review-left-ag-row-checked')).toHaveCount(1);
  }

  const tabFilter = leftPanel.getByRole('tab', { name: /过滤/ });
  const tabScreen = leftPanel.getByRole('tab', { name: /筛选/ });
  const tabSamples: number[] = [];
  for (let i = 0; i < 3; i += 1) {
    const toScreenStart = Date.now();
    await tabScreen.click();
    await expect(tabScreen).toHaveAttribute('aria-selected', 'true');
    tabSamples.push(Date.now() - toScreenStart);

    const toFilterStart = Date.now();
    await tabFilter.click();
    await expect(tabFilter).toHaveAttribute('aria-selected', 'true');
    tabSamples.push(Date.now() - toFilterStart);
  }

  const visualStats = summarize(visualSamples);
  const settleStats = summarize(settleSamples);
  const tabStats = summarize(tabSamples);

  console.log(`[perf-budget] visual=${JSON.stringify(visualStats)} budget=${visualBudgetMs}`);
  console.log(`[perf-budget] settle=${JSON.stringify(settleStats)} budget=${settleBudgetMs}`);
  console.log(`[perf-budget] tab=${JSON.stringify(tabStats)} budget=${tabBudgetMs}`);

  expect(visualStats.p95).toBeLessThanOrEqual(visualBudgetMs);
  expect(settleStats.p95).toBeLessThanOrEqual(settleBudgetMs);
  expect(tabStats.p95).toBeLessThanOrEqual(tabBudgetMs);
});
