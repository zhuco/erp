# AfterSales Receive Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the after-sales receive page automatically pick the right warehouse, surface a clear error when no warehouses are available, and keep submit state explainable across receive modes.

**Architecture:** Refactor the receive page into a small pure-model module that encapsulates default selection, validation, and submission state while keeping the React page focused on wiring props/state and user feedback.

**Tech Stack:** React (TSX), TypeScript, Vitest.

---

### Task 1: Receive page model + wiring

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/afterSalesReceiveModel.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/afterSalesReceiveModel.test.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx`

- [ ] **Step 1: Write the failing test**
  Add tests in `afterSalesReceiveModel.test.ts` that exercise `selectInitialWarehouse` (or similar) for: single warehouse auto-select, default warehouse auto-select, and the empty list rejection that blocks submit. Each test will assert experience strings for normal vs redo mode to capture explainability.

- [ ] **Step 2: Run test to verify it fails**
  Run `pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesReceiveModel.test.ts` and expect failures because the exported helpers do not yet exist or lack the needed behavior.

- [ ] **Step 3: Write minimal implementation**
  Implement pure helper(s) in `afterSalesReceiveModel.ts` that compute the initial selection, validation status, and user-facing hints. Update `AfterSalesReceivePage.tsx` to call the helpers so the page auto-selects the warehouse, shows the right message when none exist, and exposes submit enablement.

- [ ] **Step 4: Run test to verify it passes**
  Re-run `pnpm --dir apps/erp-web exec vitest run src/pages/afterSales/afterSalesReceiveModel.test.ts` and expect all tests to pass now that the helpers and page wiring exist.

- [ ] **Step 5: Commit**
  `git add apps/erp-web/src/pages/afterSales/afterSalesReceiveModel.ts apps/erp-web/src/pages/afterSales/afterSalesReceiveModel.test.ts apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx`
  `git commit -m "feat: clarify after-sales receive model"`

---

After the plan is saved, dispatch a plan-document-reviewer subagent with the plan path and spec path for review before implementation.
