# Goods Edit Workbench Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade goods product and kit editing from query-page drawers to independent industrial workbench edit routes while preserving query context on return.

**Architecture:** Add dedicated `/goods/products/:productId/edit` and `/goods/kits/:kitSkuId/edit` routes, keep the existing product and kit forms and API mappings, and rewire query pages to navigate to the new edit routes with URL-carried query context. Reuse the existing goods workbench shell, but let edit routes default to a compact workbench layout by encoding collapsed right and bottom panels in the route query string.

**Tech Stack:** React, TypeScript, React Router, TanStack Query, Ant Design, Vitest, Playwright

---

### Task 1: Lock routing, menu activation, and legacy redirects with tests

**Files:**
- Modify: `apps/erp-web/src/app/router.goods-module.test.ts`
- Modify: `apps/erp-web/src/app/routeRegistry.test.ts`
- Modify: `apps/erp-web/src/app/layout/headerMenuModel.test.ts`
- Modify: `apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts`
- Modify: `apps/erp-web/src/app/navigation/routeAliases.test.ts`
- Create: `apps/erp-web/src/app/layout/routeMeta.test.ts`
- Test: `apps/erp-web/src/app/router.goods-module.test.ts`
- Test: `apps/erp-web/src/app/routeRegistry.test.ts`
- Test: `apps/erp-web/src/app/layout/headerMenuModel.test.ts`
- Test: `apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts`
- Test: `apps/erp-web/src/app/navigation/routeAliases.test.ts`
- Test: `apps/erp-web/src/app/layout/routeMeta.test.ts`

- [ ] **Step 1: Write the failing tests**

Add expectations for:

```ts
"path: 'goods/products/:productId/edit'"
"path: 'goods/kits/:kitSkuId/edit'"
```

Extend the route registry and reduced-shell tests to include:

```ts
'/goods/products/:productId/edit'
'/goods/kits/:kitSkuId/edit'
```

Extend the header-module activation test with:

```ts
expect(resolveActiveHeaderModuleKey(modules, '/goods/products/product-1/edit')).toBe(goodsModule?.key);
expect(resolveActiveHeaderModuleKey(modules, '/goods/kits/kit-1/edit')).toBe(goodsModule?.key);
```

Update the legacy redirect expectation to:

```ts
expect(getLegacyRedirectTarget('/kits/KIT-001')).toBe('/goods/kits/KIT-001/edit');
```

Create `routeMeta.test.ts` to assert:

```ts
resolveRouteTitle('/goods/products/product-1/edit', menuTitleMap) === '货品修改'
resolveRouteTitle('/goods/kits/kit-1/edit', menuTitleMap) === '组合装修改'
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/app/router.goods-module.test.ts src/app/routeRegistry.test.ts src/app/layout/headerMenuModel.test.ts src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts src/app/navigation/routeAliases.test.ts src/app/layout/routeMeta.test.ts
```

Expected: FAIL because the new edit routes, dynamic titles, reduced-shell allowances, and legacy redirects are not implemented yet.

- [ ] **Step 3: Write the minimal implementation**

Modify:

- `apps/erp-web/src/app/router.tsx`
- `apps/erp-web/src/app/routeRegistry.ts`
- `apps/erp-web/src/app/layout/headerMenuModel.ts`
- `apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- `apps/erp-web/src/app/navigation/routeAliases.ts`
- `apps/erp-web/src/app/layout/routeMeta.ts`

Implement:

- New lazy edit pages and route entries
- Route-registry entries with `products.write` and `kits.write`
- Goods-module activation for dynamic edit paths
- Reduced-shell allowance for the two edit paths
- Legacy `/kits/:kit_sku_id` redirect to the new edit route
- Dynamic route titles `货品修改` and `组合装修改`

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/app/router.goods-module.test.ts src/app/routeRegistry.test.ts src/app/layout/headerMenuModel.test.ts src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts src/app/navigation/routeAliases.test.ts src/app/layout/routeMeta.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/app/router.tsx apps/erp-web/src/app/routeRegistry.ts apps/erp-web/src/app/layout/headerMenuModel.ts apps/erp-web/src/app/layout/WorkbenchLayout.tsx apps/erp-web/src/app/navigation/routeAliases.ts apps/erp-web/src/app/layout/routeMeta.ts apps/erp-web/src/app/router.goods-module.test.ts apps/erp-web/src/app/routeRegistry.test.ts apps/erp-web/src/app/layout/headerMenuModel.test.ts apps/erp-web/src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts apps/erp-web/src/app/navigation/routeAliases.test.ts apps/erp-web/src/app/layout/routeMeta.test.ts
git commit -m "feat: add goods workbench edit routes"
```

### Task 2: Add shared goods route helpers and lock query-context URL behavior

**Files:**
- Create: `apps/erp-web/src/pages/goods/shared/goodsRoutes.ts`
- Create: `apps/erp-web/src/pages/goods/shared/goodsRoutes.test.ts`
- Test: `apps/erp-web/src/pages/goods/shared/goodsRoutes.test.ts`

- [ ] **Step 1: Write the failing tests**

Create helper tests that lock:

```ts
buildGoodsProductEditPath('product-1', '?keyword=abc&page=2&pageSize=50&rightCollapsed=0&bottomCollapsed=0')
// => '/goods/products/product-1/edit?keyword=abc&page=2&pageSize=50&rightCollapsed=1&bottomCollapsed=1'
```

```ts
buildGoodsKitEditPath('kit-1', '?keyword=kit&activeId=kit-1')
// => '/goods/kits/kit-1/edit?keyword=kit&activeId=kit-1&rightCollapsed=1&bottomCollapsed=1'
```

```ts
buildGoodsQueryPath('/goods/products/query', '?keyword=abc&activeId=product-1')
// => '/goods/products/query?keyword=abc&activeId=product-1'
```

Also verify `edit` is removed from compatibility redirect input before generating the new edit path.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/pages/goods/shared/goodsRoutes.test.ts
```

Expected: FAIL because the helper file does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `goodsRoutes.ts` with:

- `buildGoodsProductEditPath`
- `buildGoodsKitEditPath`
- `buildGoodsQueryPath`
- `createGoodsEditSearchParams`

Behavior rules:

- Preserve existing query context
- Remove `edit`
- Force `rightCollapsed=1`
- Force `bottomCollapsed=1`

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/pages/goods/shared/goodsRoutes.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/goods/shared/goodsRoutes.ts apps/erp-web/src/pages/goods/shared/goodsRoutes.test.ts
git commit -m "feat: add goods edit route helpers"
```

### Task 3: Add independent product and kit edit workbench pages with tests first

**Files:**
- Modify: `apps/erp-web/src/pages/goods/goodsModuleSource.contract.test.ts`
- Modify: `apps/erp-web/src/pages/goods/shared/GoodsCreateWorkbench.tsx`
- Modify: `apps/erp-web/src/pages/goods/shared/goodsCreateWorkbench.css`
- Create: `apps/erp-web/src/pages/goods/product-form/GoodsProductEditPage.tsx`
- Create: `apps/erp-web/src/pages/goods/kit-form/GoodsKitEditPage.tsx`
- Test: `apps/erp-web/src/pages/goods/goodsModuleSource.contract.test.ts`

- [ ] **Step 1: Write the failing tests**

Update the goods source contract to assert:

- Query pages no longer depend on `GoodsProductEditorDialog` / `GoodsKitEditorDialog`
- Query pages reference `/edit` routes
- `GoodsProductEditPage.tsx` contains `getProduct`, `updateProduct`, and `GoodsCreateWorkbench`
- `GoodsKitEditPage.tsx` contains `getKitDetail`, `getSkuDetail`, `updateProduct`, `updateSku`, `saveKitBom`, and `GoodsCreateWorkbench`

Also add contract expectations that the shared goods workbench accepts compact edit usage via props instead of being create-only.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/pages/goods/goodsModuleSource.contract.test.ts
```

Expected: FAIL because the edit pages do not exist yet and query pages still depend on dialog editing.

- [ ] **Step 3: Write the minimal implementation**

Implement:

- `GoodsProductEditPage.tsx`
- `GoodsKitEditPage.tsx`

Reuse:

- `ProductForm`
- `KitForm`
- Existing API mapping helpers
- Existing `GoodsCreateWorkbench` with small prop extensions for compact edit presentation

Save behavior:

- Preserve form state on save failure
- On success invalidate related queries
- Return to the query URL built from the current search params

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/pages/goods/goodsModuleSource.contract.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/goods/shared/GoodsCreateWorkbench.tsx apps/erp-web/src/pages/goods/shared/goodsCreateWorkbench.css apps/erp-web/src/pages/goods/product-form/GoodsProductEditPage.tsx apps/erp-web/src/pages/goods/kit-form/GoodsKitEditPage.tsx apps/erp-web/src/pages/goods/goodsModuleSource.contract.test.ts
git commit -m "feat: add goods workbench edit pages"
```

### Task 4: Rewire query pages, legacy `?edit=` compatibility, and URL state persistence

**Files:**
- Modify: `apps/erp-web/src/pages/goods/products-query/GoodsProductsQueryPage.tsx`
- Modify: `apps/erp-web/src/pages/goods/kits-query/GoodsKitsQueryPage.tsx`
- Test: `apps/erp-web/src/pages/goods/goodsModuleSource.contract.test.ts`

- [ ] **Step 1: Write the failing test expectations**

Extend the goods source contract so that:

- Product and kit query pages contain `buildGoodsProductEditPath` / `buildGoodsKitEditPath`
- Query pages still use `useSearchParams`
- Query pages no longer render `GoodsProductEditorDialog` or `GoodsKitEditorDialog`
- Query pages still keep compatibility with `?edit=`

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/pages/goods/goodsModuleSource.contract.test.ts
```

Expected: FAIL because query pages still open drawers.

- [ ] **Step 3: Write the minimal implementation**

For both query pages:

- Replace drawer-open behavior with edit-route navigation
- Redirect old `?edit=` links to the new edit route with `replace`
- Synchronize `keyword`, filter state, pagination, and `activeId` into URL search params
- Preserve current search params when launching edit

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/pages/goods/goodsModuleSource.contract.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/goods/products-query/GoodsProductsQueryPage.tsx apps/erp-web/src/pages/goods/kits-query/GoodsKitsQueryPage.tsx apps/erp-web/src/pages/goods/goodsModuleSource.contract.test.ts
git commit -m "feat: rewire goods query pages to workbench edit routes"
```

### Task 5: Run focused verification and browser regression

**Files:**
- No code changes required

- [ ] **Step 1: Run the focused vitest suite**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/app/router.goods-module.test.ts src/app/routeRegistry.test.ts src/app/layout/headerMenuModel.test.ts src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts src/app/navigation/routeAliases.test.ts src/app/layout/routeMeta.test.ts src/pages/goods/shared/goodsRoutes.test.ts src/pages/goods/goodsModuleSource.contract.test.ts
```

Expected: PASS

- [ ] **Step 2: Run browser regression against the mock web**

Verify:

1. `/goods/products/query` supports search/filter/pagination and editing entry
2. `/goods/products/:productId/edit` loads correctly, defaults to compact layout, saves, and returns to the preserved query state
3. `/goods/kits/query` supports editing entry
4. `/goods/kits/:kitSkuId/edit` loads correctly, defaults to compact layout, saves, and returns to the preserved query state
5. Old `?edit=` goods links redirect to the new edit routes

- [ ] **Step 3: Review the diff for scope control**

Run:

```bash
git diff -- apps/erp-web/src/app/router.tsx apps/erp-web/src/app/routeRegistry.ts apps/erp-web/src/app/layout/headerMenuModel.ts apps/erp-web/src/app/layout/WorkbenchLayout.tsx apps/erp-web/src/app/navigation/routeAliases.ts apps/erp-web/src/app/layout/routeMeta.ts apps/erp-web/src/pages/goods
```

Expected: Only routing, goods workbench editing, and related tests changed.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/plans/2026-05-11-goods-edit-workbench-upgrade.md
git commit -m "docs: add goods edit workbench implementation plan"
```
