import { expect, loginAndSelectTenant, test } from './fixtures';

type PerfSummary = {
  samples: number[];
  avg: number;
  p95: number;
  max: number;
};

const summarize = (samples: number[]): PerfSummary => {
  const sorted = [...samples].sort((a, b) => a - b);
  const avg = Math.round(samples.reduce((sum, value) => sum + value, 0) / samples.length);
  const p95Index = Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95));
  return {
    samples,
    avg,
    p95: sorted[p95Index] ?? 0,
    max: sorted[sorted.length - 1] ?? 0,
  };
};

test('order review interaction latency evaluation', async ({ page }) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const leftPanel = page.getByTestId('order-review-left-ag-panel');
  const grid = page.getByTestId('order-review-data-grid');
  const loadTiming = page.getByTestId('order-review-load-timing');
  const loadCycle = page.getByTestId('order-review-load-cycle');
  await expect(leftPanel).toBeVisible();
  await expect(grid).toBeVisible();
  await expect(loadTiming).toBeVisible();
  await expect(loadCycle).toBeAttached();

  const filterSamples: number[] = [];
  const filterCases = [
    { key: 'platform:taobao', label: '淘宝' },
    { key: 'platform:jd', label: '京东' },
    { key: 'platform:pdd', label: '拼多多' },
    { key: 'platform:douyin', label: '抖音' },
  ];

  for (const item of filterCases) {
    const row = leftPanel.locator(`[row-id="${item.key}"] [role="gridcell"]`).first();
    const clearAllPlatform = leftPanel.locator('[row-id="platform:*"] [role="gridcell"]').first();
    await expect(row).toBeVisible();
    await expect(clearAllPlatform).toBeVisible();

    const start = Date.now();
    await row.click();
    const platformCells = grid.locator('.ag-center-cols-container .ag-row .ag-cell[col-id="platform"]');
    await expect
      .poll(async () => {
        const values = (await platformCells.allTextContents())
          .map((text) => text.trim())
          .filter(Boolean);
        return values.length > 0 && values.every((value) => value.includes(item.label));
      })
      .toBe(true);
    filterSamples.push(Date.now() - start);

    await clearAllPlatform.click();
  }

  const screenSamples: number[] = [];
  const tabFilter = leftPanel.getByRole('tab', { name: /过滤/ });
  const tabScreen = leftPanel.getByRole('tab', { name: /筛选/ });
  await tabScreen.click();
  await expect(tabScreen).toHaveAttribute('aria-selected', 'true');
  const createdFrom = leftPanel.getByTestId('strategy-screen-created-from');
  const createdTo = leftPanel.getByTestId('strategy-screen-created-to');
  const applyScreen = leftPanel.getByTestId('strategy-screen-apply');
  await expect(createdFrom).toBeVisible();
  await expect(createdTo).toBeVisible();
  await expect(applyScreen).toBeVisible();

  for (let i = 0; i < 3; i += 1) {
    const from = `2000-01-0${i + 1}`;
    const to = `2099-12-2${i + 1}`;
    await createdFrom.fill(from);
    await createdTo.fill(to);
    const start = Date.now();
    const cycleBefore = Number(((await loadCycle.textContent()) ?? '0').trim() || '0');
    await applyScreen.click();
    await expect
      .poll(async () => Number(((await loadCycle.textContent()) ?? '0').trim() || '0'))
      .toBeGreaterThan(cycleBefore);
    await expect(loadTiming).toContainText(/加载耗时|加载中/);
    screenSamples.push(Date.now() - start);
  }

  await tabFilter.click();
  await expect(tabFilter).toHaveAttribute('aria-selected', 'true');

  const querySamples: number[] = [];
  const orderIdCells = grid.locator('.ag-center-cols-container .ag-row .ag-cell[col-id="id"]');
  await expect.poll(async () => orderIdCells.count(), { timeout: 20_000 }).toBeGreaterThan(1);
  const firstOrderId = ((await orderIdCells.nth(0).textContent()) ?? '').trim();
  const secondOrderId = ((await orderIdCells.nth(1).textContent()) ?? '').trim();
  expect(firstOrderId).not.toBe('');
  expect(secondOrderId).not.toBe('');

  const toolbarSearchInput = page.getByTestId('order-review-toolbar-search-input');
  const toolbarSearchButton = page.getByTestId('order-review-toolbar-search-trigger');
  await expect(toolbarSearchInput).toBeVisible();
  await expect(toolbarSearchButton).toBeVisible();

  for (const keyword of [firstOrderId, secondOrderId]) {
    await toolbarSearchInput.fill(keyword);
    const start = Date.now();
    const cycleBefore = Number(((await loadCycle.textContent()) ?? '0').trim() || '0');
    await toolbarSearchButton.click();
    await expect
      .poll(async () => Number(((await loadCycle.textContent()) ?? '0').trim() || '0'))
      .toBeGreaterThan(cycleBefore);
    await expect
      .poll(async () => {
        const values = (await orderIdCells.allTextContents())
          .map((text) => text.trim())
          .filter(Boolean);
        return values.length > 0 && values.every((value) => value.includes(keyword));
      })
      .toBe(true);
    querySamples.push(Date.now() - start);
  }

  await toolbarSearchInput.fill('');
  const resetCycleBefore = Number(((await loadCycle.textContent()) ?? '0').trim() || '0');
  await toolbarSearchButton.click();
  await expect
    .poll(async () => Number(((await loadCycle.textContent()) ?? '0').trim() || '0'))
    .toBeGreaterThan(resetCycleBefore);
  await expect.poll(async () => orderIdCells.count(), { timeout: 20_000 }).toBeGreaterThan(1);

  const switchSamples: number[] = [];
  const rows = grid.locator('.ag-center-cols-container .ag-row');
  const detailLoadedId = page.getByTestId('order-review-detail-loaded-id');
  await expect(detailLoadedId).toBeAttached();

  for (let i = 0; i < 4; i += 1) {
    const targetIndex = i % 2 === 0 ? 0 : 1;
    const targetCell = orderIdCells.nth(targetIndex);
    const targetId = ((await targetCell.textContent()) ?? '').trim();
    const targetRow = rows.nth(targetIndex);
    const start = Date.now();
    await targetRow.click();
    await expect.poll(async () => ((await detailLoadedId.textContent()) ?? '').trim()).toBe(targetId);
    switchSamples.push(Date.now() - start);
  }

  const filterStats = summarize(filterSamples);
  const screenStats = summarize(screenSamples);
  const queryStats = summarize(querySamples);
  const switchStats = summarize(switchSamples);

  console.log(`[interaction-latency] filter=${JSON.stringify(filterStats)}`);
  console.log(`[interaction-latency] screen=${JSON.stringify(screenStats)}`);
  console.log(`[interaction-latency] query=${JSON.stringify(queryStats)}`);
  console.log(`[interaction-latency] switch=${JSON.stringify(switchStats)}`);

  expect(filterStats.samples.length).toBeGreaterThan(0);
  expect(screenStats.samples.length).toBeGreaterThan(0);
  expect(queryStats.samples.length).toBeGreaterThan(0);
  expect(switchStats.samples.length).toBeGreaterThan(0);
});
