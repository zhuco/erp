# Workbench Sequence Lock Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the `WorkbenchPageTemplate` sequence column keep a stable original row number for the current loaded dataset even when users sort by other columns.

**Architecture:** Store a stable shared row metadata field in the AG Grid row adaptation path, then make the structural sequence column read that metadata instead of the current displayed row index. Keep the existing sequence-column sort lock and state sanitization rules intact, and verify the behavior through shared grid and structural-column tests.

**Tech Stack:** React, TypeScript, AG Grid, Vitest

---

### Task 1: Add failing tests for stable source row metadata

**Files:**
- Modify: `apps/erp-web/src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts`
- Modify: `apps/erp-web/src/components/ErpDataGrid.columnState.test.ts`

- [ ] **Step 1: Write the failing test**

Add tests that prove:
- the structural sequence `valueGetter` prefers a stable source row field over current displayed `rowIndex`
- the structural sequence `comparator` compares stable source row values
- sequence-related persisted column state still excludes structural sort metadata

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm exec vitest --run src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts src/components/ErpDataGrid.columnState.test.ts`
Expected: FAIL because the current sequence column still depends on `rowNode.rowIndex`/`sourceRowIndex` rather than shared row metadata

- [ ] **Step 3: Write minimal implementation**

Update the structural sequence helpers to prefer a stable row metadata field such as `__erpSourceRowIndex`.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm exec vitest --run src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts src/components/ErpDataGrid.columnState.test.ts`
Expected: PASS

### Task 2: Add failing tests for AG Grid row adaptation

**Files:**
- Modify: `apps/erp-web/src/components/ErpDataGrid.tsx`
- Create or Modify: `apps/erp-web/src/components/ErpDataGrid.test.tsx`

- [ ] **Step 1: Write the failing test**

Add a focused test proving the shared AG Grid row adaptation assigns a stable `__erpSourceRowIndex` per row before client-side sorting.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm exec vitest --run src/components/ErpDataGrid.test.tsx`
Expected: FAIL because AG Grid row data does not yet include stable source row metadata

- [ ] **Step 3: Write minimal implementation**

Add shared row adaptation in `ErpDataGrid.tsx` that enriches row data with:
- `__erpRowKey`
- `__erpRowIndex`
- `__erpSourceRowIndex`

Pass the enriched rows into AG Grid without changing business-page contracts.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm exec vitest --run src/components/ErpDataGrid.test.tsx`
Expected: PASS

### Task 3: Verify workbench behavior remains locked

**Files:**
- Modify if needed: `apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts`
- Modify if needed: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts`

- [ ] **Step 1: Run the focused regression suite**

Run: `pnpm exec vitest --run src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts`
Expected: PASS or reveal any shared-policy regressions

- [ ] **Step 2: Adjust tests only if shared metadata changes helper expectations**

Keep assertions focused on:
- sequence column remains unsortable
- sequence column remains pinned/locked where required
- business-column sorting behavior stays available

- [ ] **Step 3: Re-run the focused regression suite**

Run: `pnpm exec vitest --run src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts`
Expected: PASS

### Task 4: Final verification

**Files:**
- Modify if needed: `apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify if needed: `apps/erp-web/src/pages/orders/shared/agGridWorkbenchStructuralColumns.tsx`

- [ ] **Step 1: Run the full focused regression suite**

Run: `pnpm exec vitest --run src/components/ErpDataGrid.columnState.test.ts src/components/ErpDataGrid.test.tsx src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts src/pages/orders/shared/agGridWorkbenchStructuralColumns.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: PASS

- [ ] **Step 2: Run type checking**

Run: `pnpm run typecheck`
Expected: PASS

- [ ] **Step 3: Summarize scope**

Confirm the change is shared workbench infrastructure only and that sequence numbering remains fixed only for the current loaded dataset, not across `F5` refresh or backend reloads.
