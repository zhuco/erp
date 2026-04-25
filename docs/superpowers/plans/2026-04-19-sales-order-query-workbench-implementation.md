# Sales Order Query Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a sales-facing order query workbench on top of `WorkbenchPageTemplate`, then expose it from the sales menu and router as the new standard sales order query entry.

**Architecture:** Create a dedicated sales workbench page instead of promoting the legacy sales query page as-is. Reuse existing order-query domain helpers and shared workbench controllers where they fit cleanly, but keep layout, routing, and interaction wiring aligned with the unified workbench standard.

**Tech Stack:** React 18, TypeScript, Ant Design, React Query, existing ERP workbench page templates, Vitest

---

### Task 1: Add failing adoption tests for the sales order query workbench entry

**Files:**
- Modify: `apps/erp-web/src/app/layout/menu.test.ts`
- Create: `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`
- Modify: `apps/erp-web/src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts`

- [ ] **Step 1: Write the failing tests**

Add assertions that:

- the sales menu includes `/sales/order-query`
- the sales-menu order-center leaf ordering includes the new query page before settings
- the new sales page mounts `WorkbenchPageTemplate`
- the sales page passes `leftPanel`
- the sales page does not include review / merge / split action identifiers

- [ ] **Step 2: Run tests to verify they fail**

Run:
`pnpm vitest run src/app/layout/menu.test.ts src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- FAIL because the sales route and page do not exist yet

- [ ] **Step 3: Write minimal implementation scaffolding**

Create the initial sales workbench page file and wire in the basic template declaration and menu entry placeholders needed to satisfy the adoption tests.

- [ ] **Step 4: Run tests to verify they pass**

Run:
`pnpm vitest run src/app/layout/menu.test.ts src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/app/layout/menu.test.ts apps/erp-web/src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx
git commit -m "test: add sales order query workbench adoption coverage"
```

### Task 2: Build the sales workbench page shell and sales-specific query composition

**Files:**
- Create: `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx`
- Create: `apps/erp-web/src/pages/sales/salesOrderQueryWorkbenchPage.css`
- Create: `apps/erp-web/src/pages/sales/components/SalesOrderQueryLeftPanel.tsx`
- Create: `apps/erp-web/src/pages/sales/components/salesOrderQueryLeftInteractions.ts`
- Create: `apps/erp-web/src/pages/sales/components/salesOrderQueryColumns.tsx`
- Create: `apps/erp-web/src/pages/sales/components/salesOrderQueryToolbar.tsx`

- [ ] **Step 1: Write the failing test**

Extend `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts` to assert that the page renders:

- a toolbar search area
- a workbench left panel
- a main query table region
- a right detail panel
- a bottom panel
- no review / merge / split primary actions

- [ ] **Step 2: Run test to verify it fails**

Run:
`pnpm vitest run src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- FAIL because the full sales workbench composition does not exist yet

- [ ] **Step 3: Write minimal implementation**

Build the sales workbench page on `WorkbenchPageTemplate`, including:

- search-oriented toolbar
- sales-oriented left panel
- query table with sales columns
- right detail panel using existing order detail rhythm
- bottom panel for goods / logs / package summary

Keep the page query-first and do not add review / merge / split business actions.

- [ ] **Step 4: Run test to verify it passes**

Run:
`pnpm vitest run src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx apps/erp-web/src/pages/sales/salesOrderQueryWorkbenchPage.css apps/erp-web/src/pages/sales/components/SalesOrderQueryLeftPanel.tsx apps/erp-web/src/pages/sales/components/salesOrderQueryLeftInteractions.ts apps/erp-web/src/pages/sales/components/salesOrderQueryColumns.tsx apps/erp-web/src/pages/sales/components/salesOrderQueryToolbar.tsx apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts
git commit -m "feat: add sales order query workbench shell"
```

### Task 3: Hook the page to live order-query data and shared workbench controllers

**Files:**
- Modify: `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx`
- Modify: `apps/erp-web/src/pages/sales/components/salesOrderQueryLeftInteractions.ts`
- Modify: `apps/erp-web/src/pages/sales/components/salesOrderQueryColumns.tsx`

- [ ] **Step 1: Write the failing test**

Add assertions in `apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts` that:

- the page uses shared workbench selection / view-state / interaction controllers
- the page derives active-row detail from the selected result set
- export / print / refresh actions exist
- review / merge / split shortcut wiring is absent

- [ ] **Step 2: Run test to verify it fails**

Run:
`pnpm vitest run src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- FAIL because controller wiring and action shape are not fully integrated yet

- [ ] **Step 3: Write minimal implementation**

Wire the page to:

- `useOrdersList`
- `useWorkbenchSelectionController`
- `useWorkbenchViewStateController`
- `useWorkbenchSavedViewState`
- `useWorkbenchInteractionController`
- `useWorkbenchTablePagination`

Reuse existing order detail and row-summary helpers where it keeps the page clean.

- [ ] **Step 4: Run test to verify it passes**

Run:
`pnpm vitest run src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.tsx apps/erp-web/src/pages/sales/components/salesOrderQueryLeftInteractions.ts apps/erp-web/src/pages/sales/components/salesOrderQueryColumns.tsx apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts
git commit -m "feat: wire sales order query workbench state and data"
```

### Task 4: Register the sales menu entry and router path

**Files:**
- Modify: `apps/erp-web/src/app/layout/menu.tsx`
- Modify: `apps/erp-web/src/app/layout/menu.test.ts`
- Modify: `apps/erp-web/src/app/router.tsx`

- [ ] **Step 1: Write the failing test**

Extend `apps/erp-web/src/app/layout/menu.test.ts` to assert:

- `/sales/order-query` is present in the flattened menu
- the sales module order-center group includes it

Add router assertions if there is an existing router test pattern; otherwise verify via source-level expectations in the sales page adoption test.

- [ ] **Step 2: Run test to verify it fails**

Run:
`pnpm vitest run src/app/layout/menu.test.ts src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- FAIL because the route and menu link are not fully wired

- [ ] **Step 3: Write minimal implementation**

Add:

- menu leaf at `/sales/order-query`
- lazy route import
- guarded router path using existing `orders.read` permission

- [ ] **Step 4: Run test to verify it passes**

Run:
`pnpm vitest run src/app/layout/menu.test.ts src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts`

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add apps/erp-web/src/app/layout/menu.tsx apps/erp-web/src/app/layout/menu.test.ts apps/erp-web/src/app/router.tsx apps/erp-web/src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts
git commit -m "feat: add sales menu order query workbench entry"
```

### Task 5: Run focused verification and update docs if implementation drifted

**Files:**
- Modify: `docs/superpowers/specs/2026-04-19-sales-order-query-workbench-design.md`
- Modify: `docs/superpowers/plans/2026-04-19-sales-order-query-workbench-implementation.md`

- [ ] **Step 1: Run focused unit and structure tests**

Run:
`pnpm vitest run src/app/layout/menu.test.ts src/pages/orders/shared/workbenchUnifiedTemplateAdoption.test.ts src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts src/components/page-templates/workbench/useWorkbenchSelectionController.test.ts src/components/page-templates/workbench/useWorkbenchViewStateController.test.ts`

Expected:

- PASS

- [ ] **Step 2: Run focused lint for touched files**

Run:
`pnpm exec eslint src/app/layout/menu.tsx src/app/layout/menu.test.ts src/app/router.tsx src/pages/sales/SalesOrderQueryWorkbenchPage.tsx src/pages/sales/SalesOrderQueryWorkbenchPage.test.ts src/pages/sales/components/SalesOrderQueryLeftPanel.tsx src/pages/sales/components/salesOrderQueryLeftInteractions.ts src/pages/sales/components/salesOrderQueryColumns.tsx src/pages/sales/components/salesOrderQueryToolbar.tsx`

Expected:

- PASS

- [ ] **Step 3: Update docs if the final file structure or route names changed**

Reflect any implementation drift in the spec and this plan.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-19-sales-order-query-workbench-design.md docs/superpowers/plans/2026-04-19-sales-order-query-workbench-implementation.md apps/erp-web/src/app/layout/menu.tsx apps/erp-web/src/app/layout/menu.test.ts apps/erp-web/src/app/router.tsx apps/erp-web/src/pages/sales
git commit -m "feat: complete sales order query workbench"
```
