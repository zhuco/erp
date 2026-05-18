# Sales Order Center Navigation And Recheck Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorder the top sales order-center navigation to follow the processing flow and add a sales-settings toggle for enabling order recheck without changing current routing visibility or downstream workflow behavior.

**Architecture:** Keep the change inside the existing menu and sales-settings model/page boundaries. Update menu leaf ordering so the shared top navigation inherits the new sequence automatically, then extend the sales-settings form model and summary text with a new `enableOrderRecheck` boolean that is UI-only for now.

**Tech Stack:** React, TypeScript, Ant Design, Vitest

---

### Task 1: Lock the new sales navigation order with tests

**Files:**
- Modify: `apps/erp-web/src/app/layout/menu.test.ts`
- Modify: `apps/erp-web/src/app/layout/headerMenuModel.test.ts`
- Test: `apps/erp-web/src/app/layout/menu.test.ts`
- Test: `apps/erp-web/src/app/layout/headerMenuModel.test.ts`

- [ ] **Step 1: Write the failing tests**

Update the expected flattened sales menu order to:

```ts
[
  '/orders/review',
  '/orders/recheck',
  '/sales/order-query',
  '/sales/settings',
]
```

Also update the sales header module expectation so its `defaultPath` becomes `/orders/review`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --dir apps/erp-web exec vitest --run src/app/layout/menu.test.ts src/app/layout/headerMenuModel.test.ts`

Expected: FAIL because the current menu order still starts with `/sales/order-query`.

- [ ] **Step 3: Write the minimal implementation**

Reorder the `订单中心` leaf items in:

`apps/erp-web/src/app/layout/menu.tsx`

to:

```ts
leaf('/orders/review', '订单审核', { permission: 'orders.read' }),
leaf('/orders/recheck', '订单复核', { permission: 'orders.read' }),
leaf('/sales/order-query', '订单查询', { permission: 'orders.read' }),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --dir apps/erp-web exec vitest --run src/app/layout/menu.test.ts src/app/layout/headerMenuModel.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/app/layout/menu.tsx apps/erp-web/src/app/layout/menu.test.ts apps/erp-web/src/app/layout/headerMenuModel.test.ts
git commit -m "feat: reorder sales order center navigation"
```

### Task 2: Add the sales-settings order recheck toggle with tests first

**Files:**
- Modify: `apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.test.ts`
- Create: `apps/erp-web/src/pages/sales/SalesSettingsPage.contract.test.ts`
- Modify: `apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.ts`
- Modify: `apps/erp-web/src/pages/sales/SalesSettingsPage.tsx`
- Test: `apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.test.ts`
- Test: `apps/erp-web/src/pages/sales/SalesSettingsPage.contract.test.ts`

- [ ] **Step 1: Write the failing tests**

Extend the base settings fixture with:

```ts
enableOrderRecheck: true,
```

Add expectations that the processing overview/section summary changes when `enableOrderRecheck` is `false`, reflecting that approved orders go directly to WMS shipment.

Add a source contract test that asserts `SalesSettingsPage.tsx` contains:

```ts
name="enableOrderRecheck"
label="是否启用订单复核"
```

and that the initial values object includes:

```ts
enableOrderRecheck: true,
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --dir apps/erp-web exec vitest --run src/pages/sales/salesSettingsWorkbenchModel.test.ts src/pages/sales/SalesSettingsPage.contract.test.ts`

Expected: FAIL because the new field and summary text do not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Add `enableOrderRecheck: boolean` to `SalesSettingsFormValues`.

Update:

`apps/erp-web/src/pages/sales/SalesSettingsPage.tsx`

to include:
- `enableOrderRecheck: true` in `initialValues`
- a `Switch` form item labeled `是否启用订单复核` in the `自动处理` section
- summary tags/microcopy that communicate either:
  - `订单复核已启用`
  - or `审核后直递 WMS`

Update:

`apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.ts`

so the processing overview and section detail include the recheck state.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --dir apps/erp-web exec vitest --run src/pages/sales/salesSettingsWorkbenchModel.test.ts src/pages/sales/SalesSettingsPage.contract.test.ts`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.ts apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.test.ts apps/erp-web/src/pages/sales/SalesSettingsPage.tsx apps/erp-web/src/pages/sales/SalesSettingsPage.contract.test.ts
git commit -m "feat: add sales order recheck toggle"
```

### Task 3: Run focused regression verification

**Files:**
- No code changes required

- [ ] **Step 1: Run the focused test suite**

Run:

```bash
pnpm --dir apps/erp-web exec vitest --run src/app/layout/menu.test.ts src/app/layout/headerMenuModel.test.ts src/pages/sales/salesSettingsWorkbenchModel.test.ts src/pages/sales/SalesSettingsPage.contract.test.ts
```

Expected: PASS

- [ ] **Step 2: Review diff for scope control**

Run:

```bash
git diff -- apps/erp-web/src/app/layout/menu.tsx apps/erp-web/src/app/layout/menu.test.ts apps/erp-web/src/app/layout/headerMenuModel.test.ts apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.ts apps/erp-web/src/pages/sales/salesSettingsWorkbenchModel.test.ts apps/erp-web/src/pages/sales/SalesSettingsPage.tsx apps/erp-web/src/pages/sales/SalesSettingsPage.contract.test.ts
```

Expected: Only the navigation order and sales-settings toggle/summaries changed.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/plans/2026-05-10-sales-order-center-nav-and-recheck-toggle.md
git commit -m "docs: add sales nav and recheck toggle plan"
```
