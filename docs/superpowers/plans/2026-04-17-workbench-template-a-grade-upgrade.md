# Workbench Template A-Grade Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the ERP workbench template from B-level to A-level by closing critical gaps in text integrity, accessibility, dense-data usability, and template responsiveness.

**Architecture:** Keep the existing template architecture (`WorkbenchLayout` + `WorkbenchMainTableTemplate` + `WorkbenchToolbarTemplate` + `WorkbenchListPage`) and apply focused upgrades in-place. Prioritize behavior-preserving refactors with regression tests for each gap so downstream pages inherit improvements without per-page rewrites.

**Tech Stack:** React 18, TypeScript, Ant Design, AG Grid, Vitest, Playwright

---

### Task 1: Fix Workbench Template Mojibake And Fallback Text Integrity

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/workbench/WorkbenchListPage.tsx`
- Test: `F:/erp/apps/erp-web/src/components/workbench/WorkbenchListPage.localization.test.tsx` (new)

- [ ] **Step 1: Write the failing test**

Create a localization regression test that renders fallback panels and asserts readable Chinese labels are present (not mojibake sequences).

```tsx
expect(screen.getByText('标记')).toBeInTheDocument();
expect(screen.getByText('默认过滤')).toBeInTheDocument();
expect(screen.queryByText(/鏆|琛ㄦ牸|榛樿/)).toBeNull();
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench/WorkbenchListPage.localization.test.tsx`
Expected: FAIL because current fallback strings include mojibake text.

- [ ] **Step 3: Write minimal implementation**

Replace mojibake fallback labels and placeholder descriptions in `WorkbenchListPage.tsx` with readable Chinese literals:

- `标记`, `常用功能`, `详情`
- `暂无标记`, `暂无快捷功能`, `暂无详情内容`
- `默认过滤`, `表格查询`, `暂无默认过滤`, `暂无表格查询条件`
- `请配置底部子表 Tabs`

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench/WorkbenchListPage.localization.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless user explicitly requests commit.

### Task 2: Normalize Header Search And Action Accessibility Language

**Files:**
- Modify: `F:/erp/apps/erp-web/src/app/layout/ModuleHeader.tsx`
- Test: `F:/erp/apps/erp-web/src/app/layout/ModuleHeader.a11y-labels.test.tsx` (new)

- [ ] **Step 1: Write the failing test**

Add tests asserting search input/button and dense-mode/user menu aria labels are Chinese and semantically clear.

```tsx
expect(screen.getByLabelText('功能搜索输入')).toBeInTheDocument();
expect(screen.getByLabelText('功能搜索')).toBeInTheDocument();
expect(screen.getByRole('button', { name: /紧凑模式/ })).toBeInTheDocument();
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/ModuleHeader.a11y-labels.test.tsx`
Expected: FAIL because several labels are currently English.

- [ ] **Step 3: Write minimal implementation**

Replace mixed-language aria labels with Chinese equivalents and keep intent explicit:

- `Function search input` -> `功能搜索输入`
- `Function search` -> `功能搜索`
- `Enable/Disable compact mode` -> `开启紧凑模式` / `关闭紧凑模式`
- `Notifications`, `Settings`, `Sign out` -> `通知`, `设置`, `退出登录`
- `User menu: ${userName}` -> `用户菜单：${userName}`

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/ModuleHeader.a11y-labels.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless user explicitly requests commit.

### Task 3: Restore Horizontal Usability For Wide Tables In Template Shell

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/workbench/templates/WorkbenchMainTableTemplate.css`
- Modify: `F:/erp/apps/erp-web/src/components/workbench/templates/WorkbenchMainTableTemplate.test.ts`

- [ ] **Step 1: Write the failing test**

Add a source-level regression assertion preventing `overflow-x: hidden` on table content regions in footer-enabled mode.

```ts
expect(cssSource).not.toMatch(/\.order-workbench-table-with-footer[\s\S]*overflow-x:\s*hidden/);
expect(cssSource).toMatch(/overflow-x:\s*auto/);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench/templates/WorkbenchMainTableTemplate.test.ts`
Expected: FAIL because current CSS forces horizontal overflow hidden.

- [ ] **Step 3: Write minimal implementation**

Update footer-mode table overflow rules:

- keep vertical scrolling behavior
- allow horizontal scroll (`overflow-x: auto`) on grid/table content containers
- verify no footer scrollbar sync regression in existing layout

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench/templates/WorkbenchMainTableTemplate.test.ts`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless user explicitly requests commit.

### Task 4: Add Keyboard-Accessible Splitter Semantics And Controls

**Files:**
- Modify: `F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/workbenchLayout.css`
- Test: `F:/erp/apps/erp-web/src/app/layout/WorkbenchLayout.splitter-a11y.test.tsx` (new)

- [ ] **Step 1: Write the failing test**

Add tests asserting splitters expose separator semantics and keyboard interaction:

- `role="separator"`
- `tabIndex={0}`
- directional `aria-orientation`
- `aria-valuemin` / `aria-valuemax` / `aria-valuenow`
- keyboard arrows adjust widths/heights in increments

```tsx
const left = screen.getByTestId('workbench-left-resizer');
left.focus();
await user.keyboard('{ArrowRight}');
expect(onLeftWidthChange).toHaveBeenCalled();
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/WorkbenchLayout.splitter-a11y.test.tsx`
Expected: FAIL because splitters are currently pointer-only divs.

- [ ] **Step 3: Write minimal implementation**

Add keyboard handlers and semantics for left/right/bottom splitters while preserving pointer drag behavior:

- extract shared keyboard resize helper
- wire value bounds from `workbenchPanelSizing.ts`
- ensure collapse toggles remain independent from resize focus behavior

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/WorkbenchLayout.splitter-a11y.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

Skip commit in this session unless user explicitly requests commit.

### Task 5: Increase Splitter Hit Targets And Stabilize Toolbar Responsiveness

**Files:**
- Modify: `F:/erp/apps/erp-web/src/app/layout/workbenchLayout.css`
- Modify: `F:/erp/apps/erp-web/src/components/workbench/templates/WorkbenchMainTableTemplate.css`
- Test: `F:/erp/apps/erp-web/src/components/workbench/templates/WorkbenchMainTableTemplate.responsive.test.ts` (new)
- Test: `F:/erp/apps/erp-web/e2e/workbench-main-table-visibility.spec.ts`

- [ ] **Step 1: Write the failing test**

Add CSS/layout assertions that:

- splitter hit area width/height is upgraded to operationally safe values
- toolbar center slot does not rely on absolute overlap under constrained widths

```ts
expect(css).toMatch(/\.workbench-resizer[\s\S]*width:\s*8px/);
expect(css).not.toMatch(/\.order-workbench-toolbar-center[\s\S]*position:\s*absolute/);
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench/templates/WorkbenchMainTableTemplate.responsive.test.ts`
Expected: FAIL because current splitter targets are tiny and center slot is absolute.

- [ ] **Step 3: Write minimal implementation**

Refactor responsive layout and interaction density:

- bump splitter hit area (`3px` -> `8px`+ visual/interactive split if needed)
- bump toggle control size (`12px/14px` -> at least `20px`+)
- refactor toolbar to flexible/grid layout to avoid center overlap on dense action sets

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench/templates/WorkbenchMainTableTemplate.responsive.test.ts`
Expected: PASS

- [ ] **Step 5: Run targeted e2e visual regression**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-main-table-visibility.spec.ts --project=chromium`
Expected: PASS with no layout collapse for review/query/exception pages.

- [ ] **Step 6: Commit**

Skip commit in this session unless user explicitly requests commit.

### Task 6: Add Automated Accessibility Gate For Workbench Core Paths

**Files:**
- Modify: `F:/erp/apps/erp-web/package.json`
- Modify: `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/workbench-a11y.spec.ts`
- Optional (if missing): `F:/erp/apps/erp-web/e2e/utils/a11y.ts`

- [ ] **Step 1: Write the failing test**

Add Playwright-based a11y smoke tests for `/dashboard`, `/orders/review`, `/orders` key workbench screens.

```ts
await checkA11y(page, ['.workbench-shell'], {
  detailedReport: true,
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-a11y.spec.ts --project=chromium`
Expected: FAIL before remediation (if violations exist) or blocked by missing helper/dependency.

- [ ] **Step 3: Write minimal implementation**

Install and wire `@axe-core/playwright` helper usage in e2e test suite, then remediate template-level violations surfaced by the run.

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-a11y.spec.ts --project=chromium`
Expected: PASS with no critical violations in core workbench paths.

- [ ] **Step 5: Commit**

Skip commit in this session unless user explicitly requests commit.

### Task 7: End-To-End Regression Sweep And Release Gate Notes

**Files:**
- Verify only
- Update (if needed): `F:/erp/docs/reports/workbench-template-a-grade-verification-2026-04-17.md` (new)

- [ ] **Step 1: Run focused unit suite**

Run: `pnpm --dir apps/erp-web vitest run src/components/workbench src/app/layout`
Expected: PASS

- [ ] **Step 2: Run focused e2e suite**

Run: `pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts e2e/workbench-main-table-visibility.spec.ts --project=chromium`
Expected: PASS

- [ ] **Step 3: Record residual risks**

If any unrelated pre-existing failures appear, document them explicitly in the verification report instead of claiming full green status.

- [ ] **Step 4: Commit verification notes (optional)**

Skip commit in this session unless user explicitly requests commit.

---

## Milestone Order

1. `M1 (P1 blockers):` Task 1 + Task 2 + Task 3  
2. `M2 (A11y + operability):` Task 4 + Task 5  
3. `M3 (quality gate):` Task 6 + Task 7

## Acceptance Criteria (A-Grade Exit)

- No mojibake fallback text in workbench templates.
- Header search/action labels are language-consistent and screen-reader meaningful.
- Wide-table horizontal usage is preserved in template shell.
- Splitters are pointer + keyboard operable with separator semantics.
- Splitter/toggle hit targets are operationally safe for dense daily use.
- Toolbar layout remains stable under narrow and dense action combinations.
- Workbench core paths pass dedicated automated a11y checks.
- Unit + E2E regressions for core workbench flows pass.
