# Workbench Page Template Industrial Standard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the industrial `WorkbenchPageTemplate` standard for main-table workbenches and validate it on order review/recheck with mock persistence.

**Architecture:** Keep `WorkbenchPageTemplate` as the layout shell, and add a workbench-standard behavior layer for column policy, double-click editing, persistence orchestration, and record validation. Order review and order recheck will be the first adopters, while order mock utilities enforce numbering, goods, and approval rules.

**Tech Stack:** React 18, TypeScript, Ant Design, AG Grid, Vitest, existing mock API layer

---

### Task 1: Define workbench-standard contracts

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/types.ts`
- Modify: `apps/erp-web/src/components/page-templates/workbench/index.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/workbenchContracts.test.ts`

- [ ] **Step 1: Write the failing test**

Create a test that asserts the exported workbench contract types and helper factories exist for:
- editor config
- persistence adapter
- validation result
- base field schema

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/components/page-templates/workbench/workbenchContracts.test.ts`
Expected: FAIL because the contract module does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Create `types.ts` with the shared contract types and export them through the workbench template index.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/components/page-templates/workbench/workbenchContracts.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench/types.ts apps/erp-web/src/components/page-templates/workbench/index.ts apps/erp-web/src/components/page-templates/workbench/workbenchContracts.test.ts
git commit -m "feat: add workbench template contracts"
```

### Task 2: Add column-policy normalization for all main-grid columns

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- Modify: `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx`

- [ ] **Step 1: Write the failing test**

Create tests that assert:
- sequence/system column is not sortable
- sequence/system column drops sort state
- all business columns receive a filter definition
- text/enum/date/number column defaults normalize correctly

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts`
Expected: FAIL because the normalization helper does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement the policy helper and wire it into the order review and order exception grids before AG Grid renders the columns.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.ts apps/erp-web/src/components/page-templates/workbench/workbenchGridColumnPolicy.test.ts apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx
git commit -m "feat: enforce workbench main-grid column policy"
```

### Task 3: Build the shared record-editor dialog and hook

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/WorkbenchRecordEditorDialog.tsx`
- Create: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchRecordEditor.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/WorkbenchRecordEditorDialog.test.tsx`
- Modify: `apps/erp-web/src/components/page-templates/workbench/index.ts`

- [ ] **Step 1: Write the failing test**

Create tests that assert:
- dialog opens with a base record draft
- extension field slot renders
- detail slot renders
- save button calls validation before persistence
- save error is surfaced in the dialog

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/components/page-templates/workbench/WorkbenchRecordEditorDialog.test.tsx`
Expected: FAIL because the dialog and hook do not exist.

- [ ] **Step 3: Write minimal implementation**

Implement the shared dialog, base field rendering contract, extension slot, detail slot, and save orchestration hook.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/components/page-templates/workbench/WorkbenchRecordEditorDialog.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench/WorkbenchRecordEditorDialog.tsx apps/erp-web/src/components/page-templates/workbench/useWorkbenchRecordEditor.ts apps/erp-web/src/components/page-templates/workbench/WorkbenchRecordEditorDialog.test.tsx apps/erp-web/src/components/page-templates/workbench/index.ts
git commit -m "feat: add shared workbench record editor"
```

### Task 4: Add order mock governance utilities

**Files:**
- Create: `apps/erp-web/src/pages/orders/shared/orderMockGovernance.ts`
- Test: `apps/erp-web/src/pages/orders/shared/orderMockGovernance.test.ts`
- Modify: `apps/erp-web/src/api/mocks/orders.ts`

- [ ] **Step 1: Write the failing test**

Create tests that assert:
- order numbers use `HC + date + serial`
- same-day sequence increments globally
- serial expands beyond 4 digits after `9999`
- goods count derives from `items`
- records with no goods fail approval validation

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/pages/orders/shared/orderMockGovernance.test.ts`
Expected: FAIL because the governance utility does not exist.

- [ ] **Step 3: Write minimal implementation**

Implement the utility and update mock order generation so all simulated orders use explicit items and derived goods count.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/pages/orders/shared/orderMockGovernance.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/orders/shared/orderMockGovernance.ts apps/erp-web/src/pages/orders/shared/orderMockGovernance.test.ts apps/erp-web/src/api/mocks/orders.ts
git commit -m "feat: standardize mock order numbering and goods rules"
```

### Task 5: Wire double-click editing into order review

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- Test: `apps/erp-web/src/pages/orders/OrderReviewPage.editor-flow.test.tsx`

- [ ] **Step 1: Write the failing test**

Create a test that asserts:
- double-clicking a review row opens the editor
- base fields are visible
- detail items are visible in the detail slot
- save updates the record through the mock adapter

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/pages/orders/OrderReviewPage.editor-flow.test.tsx`
Expected: FAIL because the editor flow is not wired yet.

- [ ] **Step 3: Write minimal implementation**

Connect the shared editor to the order review page using:
- workbench editor config
- order review persistence adapter
- order review validation adapter
- order item detail slot

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/pages/orders/OrderReviewPage.editor-flow.test.tsx src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/orders/OrderReviewPage.tsx apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx apps/erp-web/src/pages/orders/OrderReviewPage.editor-flow.test.tsx
git commit -m "feat: add workbench editor flow to order review"
```

### Task 6: Wire the same standard into order recheck

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx`
- Test: `apps/erp-web/src/pages/orders/OrderExceptionsPage.editor-flow.test.tsx`

- [ ] **Step 1: Write the failing test**

Create a test that asserts:
- double-clicking a recheck row opens the same shared editor
- editor save goes through the mock persistence adapter
- validation messages surface consistently

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/pages/orders/OrderExceptionsPage.editor-flow.test.tsx`
Expected: FAIL because the shared editor is not wired there yet.

- [ ] **Step 3: Write minimal implementation**

Connect the recheck page to the shared editor contract and preserve existing page-specific behavior.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/pages/orders/OrderExceptionsPage.editor-flow.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.tsx apps/erp-web/src/pages/orders/OrderExceptionsPage.editor-flow.test.tsx
git commit -m "feat: add workbench editor flow to order recheck"
```

### Task 7: Run focused regression verification

**Files:**
- Modify: `apps/erp-web/src/app/layout/reducedTemplateInventory.test.ts`
- Test: existing workbench/order focused tests

- [ ] **Step 1: Run focused lint**

Run:
`pnpm exec eslint src/components/page-templates/workbench/**/*.ts* src/pages/orders/**/*.ts* src/api/mocks/orders.ts`
Expected: PASS

- [ ] **Step 2: Run focused unit tests**

Run:
`pnpm vitest run src/components/page-templates/workbench/**/*.test.ts* src/pages/orders/**/*.test.ts* src/app/layout/reducedTemplateInventory.test.ts`
Expected: PASS

- [ ] **Step 3: Run high-signal shell tests**

Run:
`pnpm vitest run src/app/layout/WorkbenchLayout.reduced-ui-shell.test.ts src/app/layout/menu.test.ts src/app/navigation/routeAliases.test.ts`
Expected: PASS

- [ ] **Step 4: Update docs if implementation diverged**

Reflect any implementation nuance back into the spec and plan if needed.

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench apps/erp-web/src/pages/orders apps/erp-web/src/api/mocks/orders.ts apps/erp-web/src/app/layout/reducedTemplateInventory.test.ts docs/superpowers/specs/2026-04-19-workbench-page-template-industrial-standard-design.md docs/superpowers/plans/2026-04-19-workbench-page-template-industrial-standard.md
git commit -m "feat: complete industrial workbench template standard"
```
