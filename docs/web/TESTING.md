# Web UI Testing & Acceptance Guide

This document consolidates production acceptance checks and Playwright E2E guidance for the 衡策云 Web UI.

官方命令入口统一为仓库根目录 `package.json` scripts；`scripts/accept_*.sh` 与 `Makefile` 仅作为兼容层。

---

## 生产验收（官方入口）

### 一键验收

在仓库根目录运行：

```bash
pnpm run accept:web
```

兼容层（历史入口，不作为主推荐）：

```bash
bash scripts/accept_web.sh
make accept-web
```

This will run all production gates sequentially:

1. ✓ ESLint
2. ✓ TypeScript type check
3. ✓ Production build
4. (E2E tests - optional, see below)

### Prerequisites

- **Node.js**: 20.x or later
- **pnpm**: 8.x or later

Install pnpm if not already installed:

```bash
npm install -g pnpm
```

### Manual Checks

You can also run checks individually:

#### 1. Install Dependencies

```bash
cd apps/erp-web
pnpm install
```

#### 2. Linting

Check code style and catch common errors:

```bash
pnpm run lint
```

**Fix automatically:**

```bash
pnpm run lint -- --fix
```

#### 3. Type Checking

Verify TypeScript types without emitting files:

```bash
pnpm run typecheck
```

#### 4. Build

Create production bundle:

```bash
pnpm run build
```

Output will be in `apps/erp-web/dist/`

#### 5. E2E Tests (Optional)

See the E2E section below for details.

### Continuous Integration

The acceptance checks run automatically on:

- **All** `push` events
- **All** `pull_request` events

See `.github/workflows/ci.yml` for configuration.

### Troubleshooting (Acceptance)

#### ❌ "pnpm: command not found"

**Solution**: Install pnpm globally

```bash
npm install -g pnpm
```

#### ❌ Lint errors

**Common causes**:

- Unused imports
- Missing semicolons
- Incorrect indentation

**Solution**: Most issues can be auto-fixed

```bash
pnpm run lint -- --fix
```

For remaining issues, review the error output and fix manually.

#### ❌ Type check errors

**Common causes**:

- Missing type annotations
- Incorrect type usage
- OpenAPI schema out of sync

**Solution**:

1. Regenerate OpenAPI client (see below)
2. Review error messages and add/fix types
3. Check for missing dependencies

#### ❌ Build fails

**Common causes**:

- Import errors
- Environment variable issues
- Memory limits (rare)

**Solution**:

1. Check error message for specific file/line
2. Ensure all dependencies are installed
3. Try clearing build cache:

   ```bash
   rm -rf apps/erp-web/dist
   pnpm run build
   ```

### Updating OpenAPI Client

When the backend API changes (`openapi/erp_api_v1.yaml`), regenerate the TypeScript client:

#### One-Click Update

```bash
pnpm --filter erp-web generate:api
```

This will:

1. Read `openapi/erp_api_v1.yaml`
2. Generate types to `apps/erp-web/src/api/generated/schema.ts`

#### Verify the Update

After regenerating, run acceptance to ensure everything still works:

```bash
pnpm run accept:web
```

#### What to Check

- **Type errors**: New or changed API endpoints may require code updates
- **Removed endpoints**: Update code to handle deprecated APIs
- **New endpoints**: Consider adding UI for new features

### Best Practices

#### Before Committing

Always run acceptance locally before pushing:

```bash
pnpm run accept:web
```

#### Before Merging

Ensure CI passes before merging pull requests.

#### Updating Dependencies

After updating `package.json`:

1. Run `pnpm install`
2. Run acceptance checks
3. Commit both `package.json` and `pnpm-lock.yaml`

#### Code Quality

- **ESLint**: Fix linting issues before committing
- **TypeScript**: Avoid using `any`, prefer proper types
- **Tests**: Add E2E tests for critical user flows

### Performance Tips

Skip E2E tests for quick validation:

```bash
pnpm run accept:web
```

Only run E2E when:

- Significant UI changes
- Before major releases
- When explicitly requested in PR reviews

### Summary

| Check | Command | Auto-fix? |
|-------|---------|-----------|
| Lint | `pnpm run lint` | ✓ `-- --fix` |
| Type Check | `pnpm run typecheck` | ✗ |
| Build | `pnpm run build` | ✗ |
| E2E | `pnpm run test:e2e` | ✗ |
| **All** | `pnpm run accept:web` | ✗ |

### Getting Help

- **Build issues**: Check this guide first
- **CI failures**: Review GitHub Actions logs
- **Type errors**: Regenerate OpenAPI client
- **Need support**: Ask in team chat or create an issue

---

## E2E Testing (Playwright)

### Overview

The Web UI uses Playwright for end-to-end testing with mocked API responses. Tests run against actual UI components but use `VITE_USE_MOCKS=true` to avoid dependency on backend.

### Quick Start

#### Run All Tests

```bash
cd apps/erp-web
pnpm test:e2e
```

#### Run Specific Test File

```bash
pnpm exec playwright test critical-paths
```

#### Run in UI Mode (Interactive)

```bash
pnpm exec playwright test --ui
```

#### Run in Headed Mode (See Browser)

```bash
pnpm exec playwright test --headed
```

### Test Structure

#### Test Files

All E2E tests are in `apps/erp-web/e2e/`:

| File | Coverage |
|------|----------|
| `critical-paths.spec.ts` | **6 critical user paths** (new) |
| `auth-flow.spec.ts` | Login and tenant selection |
| `orders.spec.ts` | Order list and detail |
| `shipments.spec.ts` | Shipment management |
| `after-sale-cases.spec.ts` | After-sales cases |
| `kits.spec.ts` | Kit/BOM management |
| `mappings.spec.ts` | SKU mappings |
| `print-jobs.spec.ts` | Print job tracking |
| `inventory.spec.ts` | Inventory management |
| `skus.spec.ts` | SKU management |

#### Fixtures (`fixtures.ts`)

Shared test utilities:

```typescript
import { expect, test } from './fixtures';
import { loginAndSelectTenant, waitForApiRequest, MOCK_TENANT_ID } from './fixtures';
```

**Available Helpers**:

- `loginAndSelectTenant(page)` - Complete auth flow
- `waitForApiRequest(page, urlPattern)` - Wait for specific API call
- `mockPrintBridge` - Fixture to inject PrintBridge mock
- `MOCK_TENANT_ID`, `MOCK_TOKEN` - Constants for tests

### Critical User Paths

#### Path 1: Login → Tenant Selection → Order List

Tests the complete authentication and navigation flow:

```typescript
test('Path 1: Login → Tenant Selection → Order List', async ({ page }) => {
  await page.goto('/orders');
  
  // Login
  await page.getByTestId('login-token-input').fill('mock-token');
  await page.getByTestId('login-submit').click();
  
  // Select tenant
  await page.getByTestId('tenant-select').click();
  await page.getByText('主租户').click();
  await page.getByTestId('tenant-submit').click();
  
  // Verify order list
  await expect(page.getByRole('heading', { name: '订单列表' })).toBeVisible();
});
```

#### Path 2: Order List Platform Filtering

Tests platform filter and API request with correct parameters:

```typescript
test('Path 2: Order List Platform Filtering', async ({ page }) => {
  // ...login...
  
  // Intercept request
  const requestPromise = page.waitForRequest((request) => {
    return request.url().includes('platform=taobao');
  });
  
  // Select platform
  await page.getByLabel('平台').click();
  await page.getByText('淘宝').click();
  await page.getByRole('button', { name: '查询' }).click();
  
  // Verify request
  const request = await requestPromise;
  expect(request.url()).toContain('platform=taobao');
});
```

#### Path 3: Order Detail with Items Table

Tests navigation to order detail and rendering of items:

```typescript
test('Path 3: Order Detail with Items Table', async ({ page }) => {
  // ...login and navigate to orders...
  
  await page.getByTestId('order-detail-link').first().click();
  
  await expect(page.getByText('商品明细')).toBeVisible();
  await expect(page.getByText(/PLAT-SKU-\d+/)).toBeVisible();
});
```

#### Path 4: Mapping Missing → Redirect with Prefill

Tests "add mapping" workflow with prefilled query parameters:

```typescript
test('Path 4: Mapping Missing → Redirect with Prefill', async ({ page }) => {
  // ...navigate to order detail...
  
  await page.getByText('去补录映射').click();
  
  // Verify URL has prefilled params
  await expect(page).toHaveURL(/\/mappings/);
  expect(page.url()).toContain('platform=');
  expect(page.url()).toContain('shop_id=');
});
```

#### Path 5: SKU Barcode Scanner

Tests barcode scanner auto-focus, submit, and clear workflow:

```typescript
test('Path 5: SKU Barcode Scanner', async ({ page }) => {
  await page.goto('/skus');
  // ...login...
  
  const scanInput = page.getByTestId('sku-scan-input');
  await expect(scanInput).toBeFocused();
  
  await scanInput.fill('BARCODE123');
  await scanInput.press('Enter');
  
  // Verify cleared and refocused
  await expect(scanInput).toHaveValue('');
  await expect(scanInput).toBeFocused();
});
```

#### Path 6: Print with PrintBridge Mock

Tests print job creation and PrintBridge invocation:

```typescript
test('Path 6: Print with PrintBridge Mock', async ({ page, mockPrintBridge }) => {
  // ...navigate to orders...
  
  const printJobRequest = page.waitForRequest((request) => {
    return request.url().includes('/print-jobs') && request.method() === 'POST';
  });
  
  await page.getByTestId('order-print-button').first().click();
  
  // Verify print job created
  await printJobRequest;
  
  // Verify PrintBridge called
  const lastPrintPayload = await page.evaluate(() => (window as any).lastPrintPayload);
  expect(lastPrintPayload).toBeTruthy();
});
```

### Mock System

#### How Mocks Work

Tests use `VITE_USE_MOCKS=true` environment variable (set in `playwright.config.ts`) which activates mock responses in the frontend code.

**Mock locations**:

- `src/api/mocks.ts` - Mock API responses
- `src/features/printing/PrintBridge.ts` - Mock PrintBridge implementation

#### Updating Mocks

##### API Response Mocks

Edit `src/api/mocks.ts`:

```typescript
// Example: Update order mock data
export const mockOrders: OrderSummary[] = [
  {
    order_id: 'ORDER-001',
    platform: 'taobao',
    platform_order_id: 'PLAT-TAOBAO-1000',
    // ... add more fields
  }
];
```

##### PrintBridge Mock

The `mockPrintBridge` fixture injects a mock into `window`:

```typescript
window.mockPrintBridge = {
  detect: async () => ({ success: true, data: { version: '1.0.0' } }),
  printShippingLabel: async (payload) => {
    console.log('[MockPrintBridge] called with', payload);
    return { success: true };
  }
};
```

To verify PrintBridge calls in tests:

```typescript
const lastPrintPayload = await page.evaluate(() => (window as any).lastPrintPayload);
expect(lastPrintPayload).toBeTruthy();
```

### Writing New Tests

#### Template

```typescript
import { expect, test } from './fixtures';
import { loginAndSelectTenant } from './fixtures';

test('my new test', async ({ page }) => {
  await page.goto('/my-page');
  await loginAndSelectTenant(page);
  
  // Your test logic here
  await expect(page.getByText('Expected Content')).toBeVisible();
});
```

#### Best Practices

1. **Use Test IDs**: Prefer `data-testid` attributes over text or class selectors

   ```typescript
   await page.getByTestId('order-detail-link').click();
   ```

2. **Use Shared Fixtures**: Import helpers from `fixtures.ts`

   ```typescript
   import { loginAndSelectTenant } from './fixtures';
   ```

3. **Wait for API Requests**: Verify backend integration

   ```typescript
   await waitForApiRequest(page, '/api/v1/tenants');
   ```

4. **Check Visibility**: Always verify elements are visible before interacting

   ```typescript
   await expect(button).toBeVisible();
   await button.click();
   ```

5. **Use Descriptive Assertions**: Make failures easy to debug

   ```typescript
   await expect(page.getByRole('heading', { name: '订单列表' })).toBeVisible();
   ```

### Debugging Tests

#### View Test Report

After running tests:

```bash
pnpm exec playwright show-report
```

#### Debug Specific Test

```bash
pnpm exec playwright test --debug critical-paths.spec.ts
```

#### Screenshots and Videos

Failed tests automatically capture screenshots and videos in `test-results/`.

#### Browser DevTools

Run in headed mode and add `await page.pause()`:

```typescript
test('debug test', async ({ page }) => {
  await page.goto('/orders');
  await page.pause(); // Opens Playwright Inspector
});
```

### CI Integration

Tests run automatically in GitHub Actions (`.github/workflows/ci.yml`).

To enable E2E in CI, uncomment:

```yaml
- name: Install Playwright browsers
  run: pnpm --filter erp-web exec playwright install --with-deps chromium

- name: Run E2E tests
  run: pnpm --filter erp-web test:e2e
```

### Troubleshooting (E2E)

#### Test Fails with "Element not found"

**Cause**: Element not rendered or selector incorrect.

**Fix**:

1. Check `data-testid` attribute exists in component
2. Add wait: `await expect(element).toBeVisible()`
3. Run in headed mode to see UI: `pnpm exec playwright test --headed`

#### API Request Not Intercepted

**Cause**: Mock system not working or URL pattern incorrect.

**Fix**:

1. Verify `VITE_USE_MOCKS=true` in `playwright.config.ts`
2. Check mock data in `src/api/mocks.ts`
3. Check URL pattern in `waitForApiRequest()`

#### PrintBridge Mock Not Working

**Cause**: Fixture not used or window injection failed.

**Fix**:

1. Ensure test uses `mockPrintBridge` fixture:

   ```typescript
   test('my test', async ({ page, mockPrintBridge }) => { ... });
   ```

2. Check browser console for injection errors

#### Timeout Errors

**Cause**: Element takes too long to render or API slow.

**Fix**:

1. Increase timeout in test:

   ```typescript
   test.setTimeout(60_000); // 60 seconds
   ```

2. Check if mock is returning data
3. Verify network is not slow

### Performance Tips

#### Run Tests in Parallel

By default, Playwright runs tests in parallel. To control:

```bash
pnpm exec playwright test --workers=2
```

#### Run Only Changed Tests

```bash
pnpm exec playwright test --only-changed
```

#### Skip Slow Tests Locally

Mark slow tests:

```typescript
test.skip('slow test', async ({ page }) => {
  // ...
});
```

### ErpDataGrid Analysis Release Gate

For AG Grid Enterprise parity (analysis domain), run this focused gate before release.
Release gate checklist and ownership matrix: `docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md`

One-command entry (repo root):

```bash
pnpm run test:gate:grid-analysis
```

Equivalent command (inside `apps/erp-web`):

```bash
cd apps/erp-web
pnpm run test:gate:grid-analysis
```

Optional preflight (fast list check):

```bash
pnpm run test:e2e:grid-analysis -- --list
```

Recommended CI gate order:

1. `pnpm run typecheck`
2. `pnpm run test:unit:grid-analysis`
3. `pnpm run test:e2e:grid-analysis`
4. `pnpm run test:e2e:grid-performance-tier`

Scope of this gate:

- Unit: advanced filter, pivot, server-side params, export consistency, chart transform, aggregation/summary precision, and core grid hooks.
- E2E: analysis regression (`erp-data-grid-analysis.spec.ts`) + performance smoke (`erp-data-grid-performance.spec.ts`).

Release evidence template:

- `docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md` section `6. 发布证据归档模板（必填）`

### Summary

| Command | Purpose |
|---------|---------|
| `pnpm test:e2e` | Run all E2E tests |
| `pnpm exec playwright test --ui` | Interactive mode |
| `pnpm exec playwright test --headed` | See browser |
| `pnpm exec playwright test --debug` | Debug mode |
| `pnpm exec playwright show-report` | View test report |

**Key Files**:

- `e2e/critical-paths.spec.ts` - 6 critical user paths
- `e2e/fixtures.ts` - Shared test utilities
- `playwright.config.ts` - Playwright configuration
- `src/api/mocks.ts` - Mock API responses

---

**Last updated**: 2026-01-17
