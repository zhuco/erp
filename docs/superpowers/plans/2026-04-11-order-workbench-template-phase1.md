# Order Workbench Template Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 抽出完整工作台第一轮模板骨架，新增顶部工具栏模板与页面骨架模板，并让订单审核页先接入这两个模板而不改变主表、右侧详情和底部子表的内部业务逻辑。

**Architecture:** 第一轮只做“页面骨架模板 + 顶部工具栏模板”，保留订单审核现有主表、右侧详情、底部子表组件不动，避免一次性把所有交互抽象过头。模板放在 `components/workbench/templates` 下，由 `OrderReviewPage.tsx` 负责注入现有业务节点，确保模板先成为稳定骨架，再继续进入第二轮的主表模板化和第三轮的右侧/底部模板化。

**Tech Stack:** React 18、TypeScript、Ant Design 6、AG Grid Enterprise、Vitest、Playwright

---

## 文件结构

### 新建文件

- `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.tsx`
  - 完整工作台顶部工具栏模板，固定左按钮/中搜索/右通用动作三区结构
- `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.tsx`
  - 完整工作台页面骨架模板，统一承载工具栏、左侧、主区、右侧、底部
- `apps/erp-web/src/components/workbench/templates/index.ts`
  - 模板统一导出入口
- `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts`
  - 工具栏模板单元测试
- `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
  - 页面骨架模板单元测试

### 修改文件

- `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
  - 将订单审核页改造成模板消费者，接入新的 `WorkbenchToolbarTemplate` 和 `WorkbenchPageTemplate`
- `apps/erp-web/src/pages/orders/orderReviewPage.css`
  - 如有必要补充模板接入后的局部样式适配，保证现有布局不回退
- `apps/erp-web/e2e/order-review-selection.spec.ts`
  - 增加一条工作台模板装配 smoke 断言，确认顶部工具栏/主区仍按模板骨架稳定渲染

### 参考文件

- `apps/erp-web/src/components/ui-modules/UiModule02ActionSearchToolbar.tsx`
- `apps/erp-web/src/components/ui-modules/UiModule02WorkbenchTemplate.tsx`
- `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- `docs/superpowers/specs/2026-04-11-order-workbench-template-design.md`

---

### Task 1: 抽出顶部工具栏模板

**Files:**
- Create: `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.tsx`
- Create: `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts`
- Test: `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts`
- Reference: `apps/erp-web/src/components/ui-modules/UiModule02ActionSearchToolbar.tsx`

- [ ] **Step 1: 写失败的工具栏模板测试**

```ts
import { createElement } from 'react';
import { describe, expect, it, vi } from 'vitest';

import { WorkbenchToolbarTemplate } from './WorkbenchToolbarTemplate';

describe('WorkbenchToolbarTemplate', () => {
  it('keeps left / center / right toolbar structure and forwards actions', () => {
    const onSearchChange = vi.fn();
    const onSearchSubmit = vi.fn();
    const onPrint = vi.fn();
    const onExport = vi.fn();
    const onHelp = vi.fn();

    const element = WorkbenchToolbarTemplate({
      leftActions: createElement('span', { 'data-testid': 'toolbar-left' }, 'left'),
      searchValue: 'SO-001',
      onSearchChange,
      onSearchSubmit,
      searchPlaceholder: '订单号/账号/手机',
      rightActions: { onPrint, onExport, onHelp },
      searchInputTestId: 'workbench-toolbar-search-input',
      searchButtonTestId: 'workbench-toolbar-search-submit',
    });

    expect(element.props.className).toContain('order-workbench-toolbar');
  });
});
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `pnpm vitest run src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts`
Expected: FAIL，原因是 `WorkbenchToolbarTemplate.tsx` 尚不存在，或导出未定义

- [ ] **Step 3: 写最小实现**

```ts
export const WorkbenchToolbarTemplate = ({
  leftActions,
  searchValue,
  onSearchChange,
  onSearchSubmit,
  searchPlaceholder,
  rightActions,
  searchInputTestId,
  searchButtonTestId,
}: WorkbenchToolbarTemplateProps) => {
  return (
    <div className="order-workbench-toolbar">
      <div className="order-workbench-toolbar-left">{leftActions}</div>
      <div className="order-workbench-toolbar-center">...</div>
      <div className="order-workbench-toolbar-right">...</div>
    </div>
  );
};
```

实现策略：
- 不重新发明视觉结构，优先复用现有 `UiModule02ToolbarRight` 或沿用其 props 语义
- 搜索区继续保持现有中间搜索体验和 test id 协议
- 打印/导出/帮助统一走 `rightActions` 契约

- [ ] **Step 4: 跑测试，确认通过**

Run: `pnpm vitest run src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts`
Expected: PASS

- [ ] **Step 5: 提交当前增量**

```bash
git add apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.tsx \
        apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts
git commit -m "feat: add workbench toolbar template"
```

---

### Task 2: 抽出完整工作台页面骨架模板

**Files:**
- Create: `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.tsx`
- Create: `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
- Create: `apps/erp-web/src/components/workbench/templates/index.ts`
- Test: `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
- Reference: `apps/erp-web/src/components/ui-modules/UiModule02WorkbenchTemplate.tsx`

- [ ] **Step 1: 写失败的页面骨架模板测试**

```ts
import { createElement } from 'react';
import { describe, expect, it } from 'vitest';

import { WorkbenchPageTemplate } from './WorkbenchPageTemplate';

describe('WorkbenchPageTemplate', () => {
  it('renders toolbar, left, main, right, bottom and footer slots', () => {
    const element = WorkbenchPageTemplate({
      className: 'order-review-page',
      toolbar: createElement('div', { 'data-testid': 'toolbar-slot' }),
      leftPanel: createElement('div', { 'data-testid': 'left-slot' }),
      mainContent: createElement('div', { 'data-testid': 'main-slot' }),
      rightPanel: createElement('div', { 'data-testid': 'right-slot' }),
      bottomPanel: createElement('div', { 'data-testid': 'bottom-slot' }),
      footer: createElement('div', { 'data-testid': 'footer-slot' }),
    });

    expect(element.props.className).toContain('order-workbench-page');
  });
});
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `pnpm vitest run src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
Expected: FAIL，原因是 `WorkbenchPageTemplate.tsx` 尚不存在，或导出未定义

- [ ] **Step 3: 写最小实现**

```ts
export const WorkbenchPageTemplate = ({
  className,
  toolbar,
  leftPanel,
  mainContent,
  rightPanel,
  bottomPanel,
  footer,
}: WorkbenchPageTemplateProps) => {
  return (
    <div className={["order-workbench-page", className].filter(Boolean).join(' ')}>
      {toolbar}
      <div className="order-workbench-body">
        {leftPanel}
        <div className="order-workbench-main">{mainContent}</div>
        {rightPanel}
      </div>
      {bottomPanel}
      {footer}
    </div>
  );
};
```

实现策略：
- 不直接接管 `useWorkbenchPanels`，仅负责结构插槽
- 保持 `order-workbench-page` 现有类名，降低 CSS 回归风险
- 如果页面层仍需依赖原有 `setLeftPanel / setRightPanel / setBottomPanel` 机制，先允许模板只承载主页面 DOM 骨架，后续再在第二轮统一容器协议

- [ ] **Step 4: 跑测试，确认通过**

Run: `pnpm vitest run src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
Expected: PASS

- [ ] **Step 5: 提交当前增量**

```bash
git add apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.tsx \
        apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.test.ts \
        apps/erp-web/src/components/workbench/templates/index.ts
git commit -m "feat: add workbench page template"
```

---

### Task 3: 让订单审核页接入模板骨架

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/orderReviewPage.css`
- Reference: `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.tsx`
- Reference: `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.tsx`

- [ ] **Step 1: 先写失败的模板装配 smoke 断言**

在现有 `apps/erp-web/e2e/order-review-selection.spec.ts` 中新增一条断言，至少验证：

```ts
await expect(page.getByTestId('order-review-page')).toBeVisible();
await expect(page.getByTestId('order-review-toolbar-search-input')).toBeVisible();
await expect(page.getByTestId('order-review-data-grid')).toBeVisible();
```

如果需要更稳，可再补：

```ts
await expect(page.getByTestId('order-review-page').locator('.order-workbench-toolbar')).toBeVisible();
```

- [ ] **Step 2: 跑该用例，确认失败或至少不满足新模板断言**

Run: `pnpm playwright test e2e/order-review-selection.spec.ts --grep "template smoke"`
Expected: FAIL，原因是新 smoke 断言尚未满足或用例尚未新增

- [ ] **Step 3: 在页面中接入模板**

实现方向：
- 保持现有页面业务状态、数据请求、主表组件、右侧详情组件、底部子表组件不动
- 将原页面中直接写出来的：
  - `UiModule02ActionSearchToolbar`
  - 主表容器壳
  - footer 容器
  抽进 `WorkbenchToolbarTemplate` / `WorkbenchPageTemplate` 的消费方式
- 页面最终变成：

```tsx
return (
  <WorkbenchPageTemplate
    className="order-review-page"
    toolbar={<WorkbenchToolbarTemplate ... />}
    mainContent={<div className="order-review-grid-main"><OrderReviewMainGrid ... /></div>}
    footer={<UiModule03TableFooter ... />}
  />
);
```

实现约束：
- 不改变 `OrderReviewMainGrid` props
- 不改变 `OrderReviewDetailPanel` / `OrderReviewBottomPanel` 的业务逻辑
- 不改已有 test id
- 如模板引入后局部 className 结构变化，最小修改 `orderReviewPage.css` 以保持现有布局

- [ ] **Step 4: 跑类型检查和核心回归**

Run: `pnpm typecheck`
Expected: PASS

Run: `pnpm playwright test e2e/order-review-selection.spec.ts`
Expected: PASS，所有既有订单审核交互回归继续为绿

- [ ] **Step 5: 提交当前增量**

```bash
git add apps/erp-web/src/pages/orders/OrderReviewPage.tsx \
        apps/erp-web/src/pages/orders/orderReviewPage.css \
        apps/erp-web/e2e/order-review-selection.spec.ts
git commit -m "refactor: migrate order review page to workbench templates"
```

---

### Task 4: 验证模板导出与后续可复用入口

**Files:**
- Modify: `apps/erp-web/src/components/workbench/templates/index.ts`
- Optional Modify: `apps/erp-web/src/components/workbench/WorkbenchListPage.tsx`（仅在需要时补文档注释或避免命名冲突）
- Test: `apps/erp-web/src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts`
- Test: `apps/erp-web/src/components/workbench/templates/WorkbenchPageTemplate.test.ts`

- [ ] **Step 1: 补导出入口测试思路并核对路径**

确保以下导出成立：

```ts
export { WorkbenchToolbarTemplate } from './WorkbenchToolbarTemplate';
export { WorkbenchPageTemplate } from './WorkbenchPageTemplate';
```

- [ ] **Step 2: 跑单元测试全绿**

Run: `pnpm vitest run src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
Expected: PASS

- [ ] **Step 3: 跑完整第一轮验收命令**

Run: `pnpm typecheck`
Expected: PASS

Run: `pnpm vitest run src/components/workbench/templates/WorkbenchToolbarTemplate.test.ts src/components/workbench/templates/WorkbenchPageTemplate.test.ts`
Expected: PASS

Run: `pnpm playwright test e2e/order-review-selection.spec.ts`
Expected: PASS

- [ ] **Step 4: 提交第一轮模板化成果**

```bash
git add apps/erp-web/src/components/workbench/templates \
        apps/erp-web/src/pages/orders/OrderReviewPage.tsx \
        apps/erp-web/src/pages/orders/orderReviewPage.css \
        apps/erp-web/e2e/order-review-selection.spec.ts
git commit -m "feat: add workbench page and toolbar templates"
```

---

## 执行备注

- 第一轮刻意不动 `OrderReviewMainGrid.tsx` 和 `OrderReviewBottomPanel.tsx` 的内部实现，只改变它们的装配方式。
- 第二轮再继续抽 `WorkbenchMainGridTemplate<T>`，把主表交互协议上升为模板能力。
- 第三轮再抽 `WorkbenchRightPanelTemplate` 和 `WorkbenchBottomTabsTemplate`，避免一次性改动过大。
- 当前工作区不是 git workspace；如果执行时依然不是 git workspace，则提交步骤保留为计划要求，但实际执行时需跳过 commit 并在结果说明中明确记录。
- 根据当前会话约束，本轮未调度额外 reviewer subagent；若后续用户明确允许 delegation，可补一轮计划审查。
