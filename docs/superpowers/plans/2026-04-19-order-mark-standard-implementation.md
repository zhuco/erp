# Order Mark Standard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Standardize order mark display and right-panel add-mark behavior so builtin marks are display-only and only user-defined marks can be added manually.

**Architecture:** Keep `apps/erp-web/src/features/orderTags.ts` as the single business-rule center for mark parsing and editable mark menu construction. Pages and right-panel templates consume shared helpers so the main grid and right detail panel stay consistent without duplicating builtin/custom filtering logic.

**Tech Stack:** React, TypeScript, Vitest, Ant Design

---

### Task 1: Add failing tests for editable mark menu rules

**Files:**
- Modify: `apps/erp-web/src/features/orderTags.test.ts`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewDetailPanel.test.ts`

- [ ] **Step 1: Write the failing test**

Add tests that prove:
- builtin marks are excluded from the editable mark menu
- already attached custom marks are excluded from the editable mark menu
- a user-defined `VIP` mark is still addable when it is a custom tag

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm exec vitest --run src/features/orderTags.test.ts src/pages/orders/review-native/OrderReviewDetailPanel.test.ts`
Expected: FAIL because editable mark menu helper/rules are not implemented yet

- [ ] **Step 3: Write minimal implementation**

Implement a shared helper in `orderTags.ts` that builds editable right-panel mark options from available custom options and current order tags.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm exec vitest --run src/features/orderTags.test.ts src/pages/orders/review-native/OrderReviewDetailPanel.test.ts`
Expected: PASS

### Task 2: Wire the shared helper into the right detail panel

**Files:**
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewDetailPanel.tsx`
- Modify: `apps/erp-web/src/components/ui-modules/UiModule05OrderReviewRightPanelTemplate.tsx`
- Modify: `apps/erp-web/src/components/ui-modules/UiModule05OrderReviewRightPanelTemplate.test.ts`

- [ ] **Step 1: Write the failing test**

Add a right-panel test that verifies the add-mark menu only renders eligible custom marks for the current order.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm exec vitest --run src/components/ui-modules/UiModule05OrderReviewRightPanelTemplate.test.ts src/pages/orders/review-native/OrderReviewDetailPanel.test.ts`
Expected: FAIL because the page still builds mark menu items ad hoc

- [ ] **Step 3: Write minimal implementation**

Update the review detail panel to use the shared editable-mark helper and pass only filtered custom options into the right-panel template.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm exec vitest --run src/components/ui-modules/UiModule05OrderReviewRightPanelTemplate.test.ts src/pages/orders/review-native/OrderReviewDetailPanel.test.ts`
Expected: PASS

### Task 3: Verify shared display behavior remains stable

**Files:**
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.test.ts`
- Modify: `apps/erp-web/src/pages/orders/components/orderManagementColumns.test.ts`
- Modify: `apps/erp-web/src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`

- [ ] **Step 1: Run the existing focused suite**

Run: `pnpm exec vitest --run src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/components/orderManagementColumns.test.ts src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`
Expected: PASS before any refactor in display behavior

- [ ] **Step 2: Adjust tests only if shared helper integration changes inputs**

Keep the assertions focused on:
- column title is `记号`
- empty mark state does not render `-`
- builtin marks still render in display areas

- [ ] **Step 3: Re-run the focused suite**

Run: `pnpm exec vitest --run src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/components/orderManagementColumns.test.ts src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`
Expected: PASS

### Task 4: Final verification

**Files:**
- Modify if needed: `apps/erp-web/src/features/orderTags.ts`
- Modify if needed: `apps/erp-web/src/pages/orders/review-native/OrderReviewDetailPanel.tsx`

- [ ] **Step 1: Run the full focused regression suite**

Run: `pnpm exec vitest --run src/features/orderTags.test.ts src/pages/orders/review-native/OrderReviewMainGrid.test.ts src/pages/orders/components/orderManagementColumns.test.ts src/components/ui-modules/UiModule05OrderReviewRightPanelTemplate.test.ts src/pages/orders/review-native/OrderReviewDetailPanel.test.ts src/pages/orders/exceptions-native/OrderExceptionsMainGrid.test.ts`
Expected: PASS

- [ ] **Step 2: Run type checking**

Run: `pnpm run typecheck`
Expected: PASS

- [ ] **Step 3: Summarize scope**

Confirm the implementation keeps mock `F5` refresh behavior unchanged and only standardizes display/add-mark rules.
