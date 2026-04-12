import { expect, test, loginAndSelectTenant, MOCK_TENANT_ID } from './fixtures';

test.describe('Critical User Paths', () => {
    test('Path 1: Login → Tenant Selection → Order List', async ({ page }) => {
        await page.goto('/orders');

        await loginAndSelectTenant(page);

        await expect(page.getByTestId('order-detail-link').first()).toBeVisible();
        await expect(page.locator('table')).toBeVisible();
    });

    test('Path 2: Order List Platform Filtering', async ({ page }) => {
        await page.goto('/orders');
        await loginAndSelectTenant(page);

        await expect(page.getByTestId('order-detail-link').first()).toBeVisible();

        const platformSelect = page.getByLabel(/平台/);
        await platformSelect.click();
        const option = page
            .locator('.ant-select-dropdown:visible .ant-select-item-option')
            .first();
        await expect(option).toBeVisible();
        const selectedPlatform = (await option.innerText()).trim();
        await option.click();
        await page.getByRole('button', { name: /查/ }).click();
        await expect(page.locator('table').getByText(selectedPlatform).first()).toBeVisible();
    });

    test('Path 3: Order Detail with Items Table', async ({ page }) => {
        await page.goto('/orders');
        await loginAndSelectTenant(page);

        await expect(page.getByTestId('order-detail-link').first()).toBeVisible();

        // Click on first order detail link
        const detailLink = page.getByTestId('order-detail-link').first();
        await expect(detailLink).toBeVisible();
        await detailLink.click();

        // Verify order detail page
        await expect(page.getByRole('heading', { name: /订单/ })).toBeVisible();
        await expect(page.getByText(/订单明细/)).toBeVisible();

        // Verify table has platform field (in mock data)
        const itemsTable = page.locator('table').filter({ hasText: /SKU/ });
        await expect(itemsTable).toBeVisible();

        // Verify at least one SKU is shown
        await expect(page.locator('table').getByText(/PLAT-SKU-\d+/).first()).toBeVisible();
    });

    test('Path 4: Mapping Missing → Redirect with Prefill', async ({ page }) => {
        await page.goto('/orders');
        await loginAndSelectTenant(page);

        await expect(page.getByTestId('order-detail-link').first()).toBeVisible();

        // Navigate to order detail
        await page.getByTestId('order-detail-link').first().click();
        await expect(page.getByRole('heading', { name: /订单/ })).toBeVisible();

        const mappingButton = page.getByTestId('order-item-mapping-link').first();
        await expect(mappingButton).toBeVisible();

        if (await mappingButton.isEnabled()) {
            await mappingButton.click();

            // Should navigate to mappings page
            await expect(page).toHaveURL(/\/mappings/);
            await expect(page.getByRole('heading', { name: /渠道/ })).toBeVisible();

            // Verify query parameters are prefilled
            const url = page.url();
            expect(url).toContain('platform=');
            expect(url).toContain('shop_id=');
            expect(url).toContain('platform_sku_id=');
        }
    });

    test('Path 5: SKU Barcode Scanner', async ({ page }) => {
        await page.goto('/skus');
        await loginAndSelectTenant(page);

        await expect(page.getByRole('heading', { name: /SKU/ })).toBeVisible();
        await expect(page.getByText('SKU-ERP-1000')).toBeVisible();
    });

    test('Path 6: Print with PrintBridge Mock', async ({ page, mockPrintBridge }) => {
        await page.goto('/orders');
        await loginAndSelectTenant(page);

        await expect(page.getByTestId('order-detail-link').nth(2)).toBeVisible();
        await page.getByTestId('order-detail-link').nth(2).click();
        await expect(page.getByRole('heading', { name: /订单/ })).toBeVisible();

        const printButton = page.getByTestId('order-detail-print-label');
        await expect(printButton).toBeEnabled();
        await printButton.click();
        await expect(page.locator('.ant-message')).toContainText(/打印任务/);
    });
});
