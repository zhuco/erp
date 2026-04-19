# Order Workbench Industrial Table Interactions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the order workbench table interactions so left filters clear column filters, tab switches restore workbench focus/selection context, selection cells are centered, and review/recheck default columns better match ERP operations.

**Architecture:** Implement the interaction rules in the shared order workbench layer first, then connect them into `OrderReviewPage` and `OrderExceptionsPage`. Keep the column expansion aligned with fields already present in each page's summary API to avoid detail-driven N+1 requests.

**Tech Stack:** React, TypeScript, AG Grid, Ant Design, Vitest

---

### Task 1: Persist Workbench Focus And Selection State

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Test: `F:/erp/apps/erp-web/src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`

- [ ] **Step 1: Write the failing test**

Add or extend tests so saved workbench view state now expects selected keys and focused/active keys to be serialized and restored.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
Expected: FAIL because the current saved view state does not include focus/selection context.

- [ ] **Step 3: Write minimal implementation**

Extend saved-view state types and page serialization/hydration so review and recheck persist:

- selected row keys
- focused row key
- active row key / active order id
- selection anchor key where applicable

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks, because the worktree already has unrelated in-progress changes.

### Task 2: Clear Column Filters When Left Filters Apply

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/shared/useLeftFilterModule.tsx`
- Test: `F:/erp/apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsGridFilters.test.ts`
- Test: `F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts`

- [ ] **Step 1: Write the failing test**

Add focused regression coverage that left-panel leaf selection clears active main-grid column-filter state before applying the new left filter.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/exceptions-native/OrderExceptionsGridFilters.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: FAIL because the current interaction model does not fully guarantee left-filter-first clearing behavior.

- [ ] **Step 3: Write minimal implementation**

Use `onBeforeLeafFilterApply` in the shared left-filter module entry path and connect page-specific clearing logic:

- recheck clears the separate `columnFilters` object
- review clears main-grid-only column-filter fields before left filters are reduced

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/exceptions-native/OrderExceptionsGridFilters.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 3: Center Shared Selection Column Cells

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/shared/agGridWorkbenchStructuralColumns.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/orderReviewPage.css`
- Test: `F:/erp/apps/erp-web/src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts`

- [ ] **Step 1: Write the failing test**

Extend structural-column tests to assert the selection column exposes the centered header/body class contract needed by the grid.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts`
Expected: FAIL because the current shared configuration does not fully assert the centered body-cell contract.

- [ ] **Step 3: Write minimal implementation**

Update the shared structural selection column config and CSS so the selection checkbox cell content is centered consistently in the data region.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 4: Expand Review And Recheck Default Columns

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx`
- Test: `F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
- Test: `F:/erp/apps/erp-web/src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`

- [ ] **Step 1: Write the failing test**

Add column-definition expectations covering the newly agreed ERP business columns for review and the richer operational columns for recheck.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
Expected: FAIL because the current column lists are still too thin.

- [ ] **Step 3: Write minimal implementation**

Expand the column definitions with fields already present in each summary payload and keep widths/min-widths aligned with the compact workbench style.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 5: Regression Verification

**Files:**
- Verify only

- [ ] **Step 1: Run focused workbench tests**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/exceptions-native/OrderExceptionsGridFilters.test.ts src/pages/orders/shared/orderWorkbenchEntryRegression.test.ts`
Expected: PASS

- [ ] **Step 2: Run a broader order workbench regression sweep**

Run: `pnpm --dir apps/erp-web vitest run src/pages/orders`
Expected: PASS or a clearly documented list of unrelated existing failures.

- [ ] **Step 3: Record residual risk if needed**

If any unrelated failures remain, document them explicitly instead of overstating completion.
