# ErpDataGrid 开源企业级重构 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用纯开源可商用技术栈重建 `ErpDataGrid` 的状态核心与渲染适配层，并在“订单列表页”“订单审核页”完成第一批真实接入。

**Architecture:** 保留业务页面理解的 `ErpDataGrid` 产品语义，但将底层拆成“状态核心 + TanStack 适配层 + 页面接入层”。迁移顺序固定为：先建立引擎无关状态与调试页，再接入 `OrdersPage`，最后接入 `OrderReviewPage`；未迁移页面继续使用旧实现，避免一次性切换全仓。

**Tech Stack:** React、TypeScript、TanStack Table、TanStack Virtual、dnd-kit、Vitest、Playwright、Ant Design

---

## 前置说明

- 当前仓库仍有旧 `Tabulator` 实现；本计划采用“双栈过渡”，先新增 `ErpDataGridNext`，不直接替换全仓旧组件。
- 本计划只覆盖 `ErpDataGrid` 第一阶段重构与两个真实页面接入，不在本轮强制移除所有旧代码。
- 所有提交步骤默认在真实 Git 工作树或后续 worktree 中执行。

## 文件结构

### 新状态核心

- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/types.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/columnState.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/gridState.ts`

责任说明：

- `types.ts`：定义引擎无关列、单元格、选择、排序、过滤接口
- `selectionModel.ts`：管理单选、多选、Shift 连选、Ctrl/Cmd 增量选择
- `rangeModel.ts`：管理活动单元格、自由框选、复制范围
- `columnState.ts`：管理列宽、顺序、固定、显隐与持久化
- `gridState.ts`：聚合上述状态并提供统一派发入口

### 新适配层

- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/TanStackGridAdapter.tsx`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useTanStackColumns.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useVirtualRows.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useSelectionController.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useRangeController.ts`
- Create: `F:/erp/apps/erp-web/src/components/ErpDataGridNext.tsx`
- Create: `F:/erp/apps/erp-web/src/components/ErpDataGridNext.css`

责任说明：

- `TanStackGridAdapter.tsx`：渲染表头、表体、虚拟滚动与列布局
- `useSelectionController.ts`：桥接鼠标/键盘事件和行选择状态
- `useRangeController.ts`：桥接活动单元格与范围复制
- `ErpDataGridNext.tsx`：对业务页暴露的新产品层入口

### 新测试与调试页

- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/columnState.test.ts`
- Create: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.css`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`

### 第一批真实页面接入

- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrdersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewTableSection.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/orders.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-contextmenu-scroll.spec.ts`

---

### Task 1：抽离引擎无关类型与行选择模型

**Files:**
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/types.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.test.ts`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.test.ts`

- [ ] **Step 1: 先写失败的行选择测试**

```ts
import { describe, expect, it } from 'vitest';
import { reduceSelection } from './selectionModel';

describe('reduceSelection', () => {
  it('支持 Shift 连选并保持连续区间', () => {
    const result = reduceSelection({
      orderedKeys: ['r1', 'r2', 'r3', 'r4', 'r5'],
      currentKeys: ['r2'],
      anchorKey: 'r2',
      action: { type: 'shift_toggle', key: 'r5' },
    });
    expect(result.keys).toEqual(['r2', 'r3', 'r4', 'r5']);
  });
});
```

- [ ] **Step 2: 运行单测确认失败**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/selectionModel.test.ts`

Expected: FAIL，提示 `reduceSelection` 未定义或断言失败

- [ ] **Step 3: 写最小实现与引擎无关类型**

```ts
export type GridSelectionAction =
  | { type: 'replace'; key: string }
  | { type: 'toggle'; key: string }
  | { type: 'shift_toggle'; key: string };
```

```ts
export const reduceSelection = ({ orderedKeys, currentKeys, anchorKey, action }) => {
  // 先实现 replace / toggle / shift_toggle 的最小闭环
};
```

- [ ] **Step 4: 再次运行单测**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/selectionModel.test.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/components/erp-data-grid-next/core/types.ts apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.ts apps/erp-web/src/components/erp-data-grid-next/core/selectionModel.test.ts
git commit -m "feat: add engine-agnostic grid selection model"
```

### Task 2：建立范围选择与复制模型

**Files:**
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/types.ts`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.test.ts`

- [ ] **Step 1: 先写失败的范围模型测试**

```ts
import { describe, expect, it } from 'vitest';
import { createRange, expandRange, serializeRange } from './rangeModel';

describe('rangeModel', () => {
  it('支持从锚点扩展到目标单元格', () => {
    const base = createRange({ rowKey: 'r2', columnKey: 'amount' });
    const next = expandRange(base, { rowKey: 'r5', columnKey: 'status' });
    expect(serializeRange(next)).toEqual({
      startRowKey: 'r2',
      endRowKey: 'r5',
      startColumnKey: 'amount',
      endColumnKey: 'status',
    });
  });
});
```

- [ ] **Step 2: 运行单测确认失败**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/rangeModel.test.ts`

Expected: FAIL

- [ ] **Step 3: 写最小范围模型与序列化逻辑**

```ts
export const createRange = (cell) => ({ anchor: cell, focus: cell });
export const expandRange = (range, cell) => ({ ...range, focus: cell });
```

- [ ] **Step 4: 运行单测确认通过**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/rangeModel.test.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/components/erp-data-grid-next/core/types.ts apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.ts apps/erp-web/src/components/erp-data-grid-next/core/rangeModel.test.ts
git commit -m "feat: add grid range selection model"
```

### Task 3：建立列状态与持久化模型

**Files:**
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/columnState.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/columnState.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/types.ts`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/core/columnState.test.ts`

- [ ] **Step 1: 先写失败的列状态测试**

```ts
import { describe, expect, it } from 'vitest';
import { applyColumnResize, applyPinnedState, reorderColumns } from './columnState';

describe('columnState', () => {
  it('在列拖拽后保持固定列顺序', () => {
    const result = reorderColumns(
      [
        { key: 'select', pinned: 'left' },
        { key: 'order_no', pinned: null },
        { key: 'status', pinned: null },
      ],
      'status',
      'order_no',
    );
    expect(result.map((item) => item.key)).toEqual(['select', 'status', 'order_no']);
  });
});
```

- [ ] **Step 2: 运行单测确认失败**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/columnState.test.ts`

Expected: FAIL

- [ ] **Step 3: 写最小列状态实现**

```ts
export const reorderColumns = (columns, fromKey, toKey) => {
  // 先实现保持 pinned 列不被拖入非 pinned 区域的最小规则
};
```

- [ ] **Step 4: 运行单测确认通过**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/columnState.test.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/components/erp-data-grid-next/core/columnState.ts apps/erp-web/src/components/erp-data-grid-next/core/columnState.test.ts
git commit -m "feat: add grid column state model"
```

### Task 4：搭建 TanStack 适配器与新调试页

**Files:**
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/TanStackGridAdapter.tsx`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useTanStackColumns.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useVirtualRows.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useSelectionController.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid-next/engine/tanstack/useRangeController.ts`
- Create: `F:/erp/apps/erp-web/src/components/ErpDataGridNext.tsx`
- Create: `F:/erp/apps/erp-web/src/components/ErpDataGridNext.css`
- Create: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.css`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`

- [ ] **Step 1: 先写失败的 Playwright 调试页用例**

```ts
import { expect, test } from '@playwright/test';

test('ErpDataGridNext 调试页支持 Shift 连选与自由框选', async ({ page }) => {
  await page.goto('/system/erp-data-grid-next');
  await page.getByTestId('erp-grid-next-row-checkbox-r2').click();
  await page.keyboard.down('Shift');
  await page.getByTestId('erp-grid-next-row-checkbox-r5').click();
  await page.keyboard.up('Shift');
  await expect(page.getByTestId('erp-grid-next-selected-count')).toHaveText('4');
});
```

- [ ] **Step 2: 运行 E2E 确认失败**

Run: `pnpm --filter erp-web test:e2e -- --grep "ErpDataGridNext"`

Expected: FAIL，提示路由或选择器不存在

- [ ] **Step 3: 写最小 TanStack 适配器与调试页**

```tsx
export const ErpDataGridNext = <T,>(props: ErpDataGridNextProps<T>) => {
  return <TanStackGridAdapter {...props} />;
};
```

要求：

- 支持基础列渲染
- 支持虚拟滚动
- 支持复选框列
- 支持活动单元格和范围状态显示

- [ ] **Step 4: 运行调试页 E2E**

Run: `pnpm --filter erp-web test:e2e -- --grep "ErpDataGridNext"`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/components/ErpDataGridNext.tsx apps/erp-web/src/components/ErpDataGridNext.css apps/erp-web/src/components/erp-data-grid-next apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.tsx apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.css apps/erp-web/src/app/router.tsx apps/erp-web/src/app/layout/menu.tsx apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts
git commit -m "feat: add TanStack-based ErpDataGridNext debug page"
```

### Task 5：接入订单列表页

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrdersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/OrderQuickFilterCells.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/orders.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/orders.spec.ts`

- [ ] **Step 1: 先写失败的订单列表页交互断言**

```ts
test('订单列表页在排序后保持选中集合', async ({ page }) => {
  await page.goto('/orders');
  await page.getByTestId('orders-grid-row-checkbox-1').click();
  await page.getByTestId('orders-grid-sort-created-at').click();
  await expect(page.getByTestId('orders-grid-selected-count')).toHaveText('1');
});
```

- [ ] **Step 2: 运行页面 E2E 确认失败**

Run: `pnpm --filter erp-web test:e2e -- apps/erp-web/e2e/orders.spec.ts`

Expected: FAIL

- [ ] **Step 3: 将 `OrdersPage` 切到 `ErpDataGridNext`**

```tsx
import { ErpDataGridNext } from '@/components/ErpDataGridNext';
```

要求：

- 保持现有筛选栏、导出、状态栏能力
- 用新列状态模型替换旧引擎绑定逻辑
- 让选中集合与排序、过滤解耦

- [ ] **Step 4: 重跑订单列表页 E2E**

Run: `pnpm --filter erp-web test:e2e -- apps/erp-web/e2e/orders.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/pages/orders/OrdersPage.tsx apps/erp-web/src/components/OrderQuickFilterCells.tsx apps/erp-web/e2e/orders.spec.ts
git commit -m "feat: migrate orders page to ErpDataGridNext"
```

### Task 6：接入订单审核页

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/components/OrderReviewTableSection.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-contextmenu-scroll.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`

- [ ] **Step 1: 先写失败的审核页选择与右键断言**

```ts
test('订单审核页右键菜单作用于当前选中行集合', async ({ page }) => {
  await page.goto('/orders/review');
  await page.getByTestId('order-review-row-checkbox-1').click();
  await page.getByTestId('order-review-row-checkbox-2').click({ modifiers: ['Control'] });
  await page.getByTestId('order-review-row-2').click({ button: 'right' });
  await expect(page.getByTestId('order-review-context-selected-count')).toHaveText('2');
});
```

- [ ] **Step 2: 运行审核页 E2E 确认失败**

Run: `pnpm --filter erp-web test:e2e -- apps/erp-web/e2e/order-review-selection.spec.ts`

Expected: FAIL

- [ ] **Step 3: 将审核主表切到 `ErpDataGridNext`**

要求：

- 行选择与范围状态分离
- 右键菜单只读行选择集合
- 滚动、刷新、详情联动不破坏活动行

必要最小代码骨架：

```tsx
<ErpDataGridNext
  data={rows}
  columns={columns}
  selectedRowKeys={selectedRowKeys}
  onSelectionChange={handleSelectionChange}
  onRowContextMenu={handleContextMenu}
/>
```

- [ ] **Step 4: 重跑审核页 E2E**

Run: `pnpm --filter erp-web test:e2e -- apps/erp-web/e2e/order-review-selection.spec.ts`

Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add apps/erp-web/src/pages/orders/OrderReviewPage.tsx apps/erp-web/src/pages/orders/components/OrderReviewTableSection.tsx apps/erp-web/e2e/order-review-selection.spec.ts apps/erp-web/e2e/order-review-contextmenu-scroll.spec.ts
git commit -m "feat: migrate order review page to ErpDataGridNext"
```

### Task 7：补最终回归并记录双栈边界

**Files:**
- Modify: `F:/erp/docs/ops/ENGINEERING_STANDARDS.md`
- Modify: `F:/erp/docs/superpowers/specs/2026-03-24-project-standardization-and-erpdatagrid-design.md`
- Test: `F:/erp/apps/erp-web/e2e/erp-data-grid-next-debug.spec.ts`

- [ ] **Step 1: 运行第一阶段全部关键测试**

Run: `pnpm --filter erp-web test:unit -- src/components/erp-data-grid-next/core/*.test.ts`

Expected: PASS

- [ ] **Step 2: 运行调试页与两个真实页面 E2E**

Run: `pnpm --filter erp-web test:e2e -- --grep "ErpDataGridNext|订单列表页|订单审核页"`

Expected: PASS

- [ ] **Step 3: 运行类型检查与构建**

Run: `pnpm --filter erp-web typecheck && pnpm --filter erp-web build`

Expected: PASS

- [ ] **Step 4: 在文档中记录“双栈过渡”规则**

```md
## ErpDataGrid 双栈过渡规则

- 已迁移页面使用 `ErpDataGridNext`
- 未迁移页面继续使用旧 `ErpDataGrid`
- 未完成全量迁移前，不再给旧 `Tabulator` 实现新增长期能力
```

- [ ] **Step 5: 提交**

```bash
git add docs/ops/ENGINEERING_STANDARDS.md docs/superpowers/specs/2026-03-24-project-standardization-and-erpdatagrid-design.md
git commit -m "docs: record ErpDataGrid dual-stack migration rules"
```
