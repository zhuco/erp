# Shop Product Download Workbench Deep-Clone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the shop product download page so its left panel, main work area, right detail panel, and bottom panel behave like the order review workbench while keeping shop-product business actions unchanged.

**Architecture:** Keep all fetching, mutations, modal state, and workbench orchestration in `ShopProductDownloadPage.tsx`, but make the left, main, right, and bottom areas true local workbench components with explicit contracts. Lock the “active row vs selected rows” rules in a pure model helper first, then reshape the page so right-side details follow the active row and the bottom area carries persistent task/failure/batch context.

**Tech Stack:** React 18, TypeScript, Ant Design, TanStack Query, Vitest, Playwright

---

## File Structure

- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/shopProductDownloadPage.css`
- Modify: `apps/erp-web/src/pages/shopProducts/shopProductWorkbenchModel.ts`
- Modify: `apps/erp-web/src/pages/shopProducts/shopProductWorkbenchModel.test.ts`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchLeftPanel.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchMainTable.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchDetailPanel.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchBottomPanel.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
- Modify: `apps/erp-web/e2e/shop-product-match-governance.spec.ts`

### Task 1: Lock workbench state and element contracts in tests

**Files:**
- Modify: `apps/erp-web/src/pages/shopProducts/shopProductWorkbenchModel.test.ts`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`

- [ ] **Step 1: Write the failing tests**

Cover:
- active row remains when still present
- active row falls back to the first current-page row when its record disappears
- active row clears on empty results
- overview counts split matched, unmatched, conflict, selected
- left panel renders order-review-like dual tabs with filter and overview content
- detail panel renders structured identity and workbench status content
- bottom panel renders task center, failures, and batch context tabs
- main table keeps `OrderWorkbenchMainGridEntry` and query variant-based workbench rendering

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/pages/shopProducts/shopProductWorkbenchModel.test.ts src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
Expected: FAIL because the current implementation still reflects the lighter workbench version.

- [ ] **Step 3: Write the minimal implementation hooks**

Implement only the minimum test-facing contracts:
- helper outputs for overview and active-row fallback
- stable `data-testid` hooks and expected labels in panel components
- template assertions for the main table contract

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/pages/shopProducts/shopProductWorkbenchModel.test.ts src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit during in-flight workspace work unless explicitly requested.

### Task 2: Deep-clone the left, main, right, and bottom workbench elements

**Files:**
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchLeftPanel.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchMainTable.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchDetailPanel.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchBottomPanel.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/shopProductDownloadPage.css`

- [ ] **Step 1: Write or tighten failing tests first**

Extend the unit tests from Task 1 so they demand the deeper structure:
- left panel overview hierarchy and helper text
- detail panel status guidance for unmatched/matched/conflict records
- bottom panel persistent task/failure/batch sections
- main table active-row styling hooks

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm vitest run src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
Expected: FAIL until the deeper workbench structure is implemented.

- [ ] **Step 3: Write minimal implementation**

Implement:
- left panel as a true filter/overview workbench panel
- main table behavior and class hooks aligned to order-review workbench usage
- right detail panel with order-review-style structured slots and state guidance
- bottom panel as a workbench bottom container rather than generic tabs
- CSS updates for workbench density, active-row emphasis, and bottom-panel layout

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm vitest run src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit during in-flight workspace work unless explicitly requested.

### Task 3: Rebuild the page orchestration around the deep-clone workbench flow

**Files:**
- Modify: `apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
- Modify: `apps/erp-web/src/pages/shopProducts/shopProductWorkbenchModel.ts`
- Modify: `apps/erp-web/e2e/shop-product-match-governance.spec.ts`

- [ ] **Step 1: Write the failing test**

Adjust the targeted E2E coverage so it asserts the intended page flow:
- workbench layout is present
- row activation drives right-side details
- selection drives batch context without stealing detail focus
- existing modals still open from the toolbar actions

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm playwright test e2e/shop-product-match-governance.spec.ts --project=chromium`
Expected: FAIL after the spec expects the new workbench flow but before implementation is complete.

- [ ] **Step 3: Write minimal implementation**

Implement:
- page state orchestration for filters, pagination, active row, selected rows, and failures
- left-panel injection through `useWorkbenchPanels`
- right detail binding to active row only
- bottom panel binding to task/failure/batch context
- existing download/create/manual-match/delete flows preserved in the toolbar and modals

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm playwright test e2e/shop-product-match-governance.spec.ts --project=chromium`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit during in-flight workspace work unless explicitly requested.

### Task 4: Final verification

**Files:**
- Modify only if fixes are needed

- [ ] **Step 1: Run targeted unit tests**

Run: `pnpm vitest run src/pages/shopProducts/shopProductWorkbenchModel.test.ts src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
Expected: PASS

- [ ] **Step 2: Run targeted app typecheck or equivalent validation**

Run: `pnpm --filter erp-web exec tsc --noEmit`
Expected: PASS

- [ ] **Step 3: Run the targeted governance E2E**

Run: `pnpm playwright test e2e/shop-product-match-governance.spec.ts --project=chromium`
Expected: PASS

- [ ] **Step 4: Fix regressions and rerun failed checks**

- [ ] **Step 5: Commit**

Skip commit during in-flight workspace work unless explicitly requested.
