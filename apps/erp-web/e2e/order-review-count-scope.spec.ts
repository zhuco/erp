import { expect, loginAndSelectTenant, test } from './fixtures';

const parseTrailingCount = (text: string) => {
  const match = text.match(/(\d+)\s*$/);
  return match ? Number(match[1]) : NaN;
};

test('order review left time counts use same scope as page total', async ({ page }, testInfo) => {
  await page.goto('/orders/review');
  await loginAndSelectTenant(page);

  const leftPanel = page.getByTestId('order-review-left-ag-panel');
  const pageRoot = page.getByTestId('order-review-page');
  await expect(leftPanel).toBeVisible();
  await expect(pageRoot).toBeVisible();

  const totalText = ((await leftPanel.getByTestId('order-review-left-stats-total').textContent()) ?? '').trim();
  const totalCount = parseTrailingCount(totalText);

  const row30 = leftPanel.locator('.order-review-left-ag-grid-shell .ag-row[row-id="time:within30d"]').first();
  await expect(row30).toBeVisible();
  const row30Text = ((await row30.textContent()) ?? '').trim();
  const row30Count = parseTrailingCount(row30Text);

  const footerSummary = pageRoot.getByText(/总数\s*\d+\s*条，本页\s*\d+\s*条/).first();
  await expect(footerSummary).toBeVisible();
  const footerText = ((await footerSummary.textContent()) ?? '').trim();
  const footerMatch = footerText.match(/总数\s*(\d+)\s*条/);
  const footerTotal = footerMatch ? Number(footerMatch[1]) : NaN;

  console.log(
    `[count-scope-repro] total=${totalCount} within30d=${row30Count} footerTotal=${footerTotal} totalText="${totalText}" row30Text="${row30Text}" footerText="${footerText}"`,
  );

  const shot = testInfo.outputPath('order-review-count-scope-repro.png');
  await leftPanel.screenshot({ path: shot });
  console.log(`[count-scope-repro] screenshot=${shot}`);

  expect(Number.isFinite(totalCount)).toBe(true);
  expect(Number.isFinite(row30Count)).toBe(true);
  expect(Number.isFinite(footerTotal)).toBe(true);
  expect(row30Count).toBeLessThanOrEqual(totalCount);
  expect(footerTotal).toBe(totalCount);
});
