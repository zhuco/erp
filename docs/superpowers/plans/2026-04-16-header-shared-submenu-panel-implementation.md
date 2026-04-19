# Header Shared Submenu Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the header menu's per-trigger submenu popups with one shared submenu panel whose left/right boundaries stay fixed to the header menu area and whose height alone changes by module.

**Architecture:** Extract the top-level menu model into explicit module metadata, render the primary header navigation in a controlled way inside `AppLayout`, and mount a single submenu panel container anchored to the header menu region. Drive open state, geometry, and height transitions from React state so different modules reuse the same panel instead of creating separate Ant Design popups.

**Tech Stack:** React, TypeScript, Ant Design, Vitest, Playwright

---

### Task 1: Add Shared Menu Model Coverage

**Files:**
- Create: `F:/erp/apps/erp-web/src/app/layout/headerMenuModel.test.ts`
- Create: `F:/erp/apps/erp-web/src/app/layout/headerMenuModel.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`

- [ ] **Step 1: Write the failing test**

Add a focused model test that expects:

- route-to-module resolution for current paths
- module extraction that preserves grouped submenu content
- leaf-only modules to report no shared panel groups

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts`
Expected: FAIL because the shared header menu model file does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Create `headerMenuModel.ts` with helpers to:

- normalize `AppMenuItem[]` into top-level header modules
- resolve a pathname to its owning top-level module
- expose grouped submenu data for the shared panel

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 2: Add Header Shared Panel Interaction Tests

**Files:**
- Create: `F:/erp/apps/erp-web/src/app/layout/headerSharedSubmenu.test.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/AppLayout.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/appLayout.css`

- [ ] **Step 1: Write the failing test**

Add component-level tests for:

- opening a shared panel from a grouped module
- reusing the same panel when switching modules
- closing on repeat click and `Escape`
- direct navigation for modules without submenu groups

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerSharedSubmenu.test.tsx`
Expected: FAIL because the current header still uses Ant Design's default submenu popup behavior and does not expose the shared-panel contract.

- [ ] **Step 3: Write minimal implementation**

Refactor `AppLayout.tsx` so the header renders a controlled primary navigation and a single shared submenu panel with stable test ids.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerSharedSubmenu.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 3: Implement Fixed-Boundary Panel Geometry And Height Transitions

**Files:**
- Modify: `F:/erp/apps/erp-web/src/app/layout/AppLayout.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/appLayout.css`

- [ ] **Step 1: Write the failing test**

Extend the shared submenu interaction test so it expects:

- one shared panel container only
- stable `data-left`/`data-width` style contract or equivalent measurable geometry metadata
- panel height to update when switching to a module with different submenu content

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerSharedSubmenu.test.tsx`
Expected: FAIL because the current panel does not measure and preserve a single anchored geometry contract.

- [ ] **Step 3: Write minimal implementation**

Add:

- a menu-area anchor `ref`
- stable shared panel geometry state derived from the menu-area rect
- content height measurement and transition styles
- outside-click and resize synchronization

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerSharedSubmenu.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 4: Add Geometry-Level Playwright Regression

**Files:**
- Modify: `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/AppLayout.tsx`

- [ ] **Step 1: Write the failing test**

Add a Playwright regression that:

- opens two different grouped modules
- reads the shared panel bounding box each time
- asserts `left`, `right`, and `top` remain stable while `height` changes

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts --project=chromium -g "shared submenu panel"`
Expected: FAIL because the current header does not expose one shared fixed-boundary submenu panel.

- [ ] **Step 3: Write minimal implementation**

Add any missing test ids and stabilize the DOM contract so Playwright can measure the shared panel and the menu-area anchor reliably.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts --project=chromium -g "shared submenu panel"`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless the user explicitly asks.

### Task 5: Verification Sweep

**Files:**
- Verify only

- [ ] **Step 1: Run focused layout unit tests**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/menu.test.ts src/app/layout/moduleSearch.test.ts src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.tsx`
Expected: PASS

- [ ] **Step 2: Run focused E2E regression**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts --project=chromium -g "shared submenu panel"`
Expected: PASS

- [ ] **Step 3: Record residual risks if any remain**

If geometry or responsive behavior still has a known edge case, document it explicitly instead of overstating completion.
