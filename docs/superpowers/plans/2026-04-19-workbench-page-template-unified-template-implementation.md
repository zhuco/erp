# Workbench Page Template Unified Template Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `WorkbenchPageTemplate` into the single reusable ERP workbench template by standardizing panel composition, workbench state, shortcut/context-menu orchestration, and validation-page adoption.

**Architecture:** Keep `WorkbenchPageTemplate` as the only public page entry, but move shared interaction behavior into internal workbench contracts and controllers. Order review and order recheck will migrate to the new declarative API so future pages can reuse the same template contract without copying page-local workbench runtime logic.

**Tech Stack:** React 18, TypeScript, Ant Design, AG Grid, Vitest, existing ERP layout/panel infrastructure

---

### Task 1: Define unified workbench contracts and public template API

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/contracts.ts`
- Modify: `apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx`
- Modify: `apps/erp-web/src/components/page-templates/workbench/index.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/workbenchContracts.test.ts`

- [ ] **Step 1: Write the failing test**

Create tests that assert the workbench module exports contract types and helpers for:
- left/right/bottom panel composition
- context-menu state
- keyboard shortcuts
- workbench selection state

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm vitest run src/components/page-templates/workbench/workbenchContracts.test.ts`
Expected: FAIL because the new contract module and API surface do not exist.

- [ ] **Step 3: Write minimal implementation**

Create the shared contracts file, extend `OrderReviewWorkbenchTemplateProps` to include the unified panel and interaction inputs, and export the new contracts through the workbench page-template index.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm vitest run src/components/page-templates/workbench/workbenchContracts.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench/contracts.ts apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx apps/erp-web/src/components/page-templates/workbench/index.ts apps/erp-web/src/components/page-templates/workbench/workbenchContracts.test.ts
git commit -m "feat: add unified workbench template contracts"
```

### Task 2: Build shared layout and interaction controllers

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts`
- Create: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchInteractionController.ts`
- Modify: `apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx`
- Test: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.test.tsx`
- Test: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchInteractionController.test.tsx`

- [ ] **Step 1: Write the failing tests**

Create tests that assert:
- `leftPanel` mounts through the standard template path
- panel visibility toggles cleanly on mount/unmount
- keyboard shortcuts ignore editable targets
- context-menu state opens/closes with the provided record payload

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/components/page-templates/workbench/useWorkbenchLayoutController.test.tsx src/components/page-templates/workbench/useWorkbenchInteractionController.test.tsx`
Expected: FAIL because the controllers do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement the layout controller for left/right/bottom panel mounting and the interaction controller for shortcut/context-menu lifecycle, then wire both into the template shell.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/components/page-templates/workbench/useWorkbenchLayoutController.test.tsx src/components/page-templates/workbench/useWorkbenchInteractionController.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.ts apps/erp-web/src/components/page-templates/workbench/useWorkbenchInteractionController.ts apps/erp-web/src/components/workbench/templates/OrderReviewWorkbenchTemplate.tsx apps/erp-web/src/components/page-templates/workbench/useWorkbenchLayoutController.test.tsx apps/erp-web/src/components/page-templates/workbench/useWorkbenchInteractionController.test.tsx
git commit -m "feat: add workbench layout and interaction controllers"
```

### Task 3: Standardize shared workbench view state and selection lifecycle

**Files:**
- Create: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchSelectionController.ts`
- Create: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchViewStateController.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchSelectionController.test.ts`
- Test: `apps/erp-web/src/components/page-templates/workbench/useWorkbenchViewStateController.test.ts`

- [ ] **Step 1: Write the failing tests**

Create tests that assert:
- selected rows are trimmed when filtered data removes them
- active/focused row falls back predictably when data changes
- persisted workbench state hydrates selection/focus/active/filter payloads
- view-state serialization preserves only the configured workbench fields

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/components/page-templates/workbench/useWorkbenchSelectionController.test.ts src/components/page-templates/workbench/useWorkbenchViewStateController.test.ts`
Expected: FAIL because the shared controllers do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement the shared selection and view-state controllers with row-key based coordination and persistence helpers for validation-page adoption.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/components/page-templates/workbench/useWorkbenchSelectionController.test.ts src/components/page-templates/workbench/useWorkbenchViewStateController.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/components/page-templates/workbench/useWorkbenchSelectionController.ts apps/erp-web/src/components/page-templates/workbench/useWorkbenchViewStateController.ts apps/erp-web/src/components/page-templates/workbench/useWorkbenchSelectionController.test.ts apps/erp-web/src/components/page-templates/workbench/useWorkbenchViewStateController.test.ts
git commit -m "feat: add shared workbench state controllers"
```

### Task 4: Migrate order review to the unified template standard

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/shared/useLeftFilterModule.tsx`
- Modify: `apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx`
- Test: `apps/erp-web/src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts`
- Test: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts`

- [ ] **Step 1: Write or extend the failing tests**

Add assertions that order review:
- passes `leftPanel` through `WorkbenchPageTemplate`
- uses shared shortcut/context-menu orchestration
- preserves selection/focus/active-row behavior through the shared controllers

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: FAIL because the page still owns the old workbench runtime wiring.

- [ ] **Step 3: Write minimal implementation**

Refactor order review to consume the unified template API and shared workbench controllers while leaving review/split/merge business actions page-owned.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/orders/OrderReviewPage.tsx apps/erp-web/src/pages/orders/shared/useLeftFilterModule.tsx apps/erp-web/src/pages/orders/shared/OrderWorkbenchMainGridEntry.tsx apps/erp-web/src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts
git commit -m "feat: migrate order review to unified workbench template"
```

### Task 5: Migrate order recheck to the same unified template standard

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/exceptions-native/useOrderExceptionsLeftModule.tsx`
- Modify: `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`
- Test: `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`

- [ ] **Step 1: Write or extend the failing tests**

Add assertions that order recheck:
- uses `leftPanel` through the template
- uses shared selection/view-state behavior
- preserves current context-menu and editor wiring through the unified interaction contract

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`
Expected: FAIL because the page is still wired to the previous page-local workbench runtime.

- [ ] **Step 3: Write minimal implementation**

Refactor order recheck to consume the same unified template API and shared workbench controllers while keeping page-specific recheck actions in the page.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx apps/erp-web/src/pages/orders/exceptions-native/useOrderExceptionsLeftModule.tsx apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts
git commit -m "feat: migrate order recheck to unified workbench template"
```

### Task 6: Run focused verification and refresh docs if needed

**Files:**
- Modify: `docs/superpowers/specs/2026-04-19-workbench-page-template-unified-template-design.md`
- Modify: `docs/superpowers/plans/2026-04-19-workbench-page-template-unified-template-implementation.md`

- [ ] **Step 1: Run focused unit and component tests**

Run:
`pnpm vitest run src/components/page-templates/workbench/**/*.test.ts* src/pages/orders/**/*workbench*.test.ts* src/pages/orders/**/*MainGrid.test.ts`
Expected: PASS

- [ ] **Step 2: Run focused workbench regression tests**

Run:
`pnpm vitest run src/pages/orders/OrderReviewPage.keyboard-efficiency.test.ts src/components/workbench/templates/OrderReviewWorkbenchTemplate.typography.test.ts src/app/layout/WorkbenchLayout.industrial-upgrade.test.ts`
Expected: PASS

- [ ] **Step 3: Run high-signal E2E coverage if runtime allows**

Run:
`pnpm exec playwright test e2e/order-review-selection.spec.ts e2e/order-review-interactions.spec.ts --project=chromium`
Expected: PASS

- [ ] **Step 4: Update docs if implementation deviated from plan/spec**

Reflect any final API or rollout adjustments in the spec and this plan.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-19-workbench-page-template-unified-template-design.md docs/superpowers/plans/2026-04-19-workbench-page-template-unified-template-implementation.md apps/erp-web/src/components/page-templates/workbench apps/erp-web/src/components/workbench/templates apps/erp-web/src/pages/orders
git commit -m "feat: complete unified workbench template standard"
```
