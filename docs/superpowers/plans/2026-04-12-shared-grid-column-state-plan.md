# 共享表格列控制与持久化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 AG Grid 原生列菜单中文化、表头右键列控制、列状态持久化下沉到共享 `ErpDataGrid`，并让订单查询、网店订单查询、物流查询及订单审核对齐同一套列交互协议。

**Architecture:** 以 `ErpDataGrid` 作为共享能力下沉点，新增独立的列菜单/列状态工具函数，页面只提供 `columnSettingsKey` 或页面级 key。`ErpDataGridNext` 与 `UiModule03MainTableSection` 不重复实现能力，只负责透传，订单审核通过复用共享工具保持行为一致。

**Tech Stack:** React, TypeScript, AG Grid Enterprise, Vitest, Playwright

---

### Task 1: 锁住共享表格行为测试

**Files:**
- Create: `apps/erp-web/src/components/ErpDataGrid.columnState.test.tsx`
- Modify: `apps/erp-web/e2e/orders.spec.ts`

- [ ] **Step 1: 写失败测试，覆盖中文菜单映射与列状态保存/恢复**
- [ ] **Step 2: 运行 `pnpm vitest run src/components/ErpDataGrid.columnState.test.tsx`，确认因功能缺失而失败**
- [ ] **Step 3: 补一条共享链路 E2E，验证列显隐刷新后仍保留**
- [ ] **Step 4: 运行 `pnpm playwright test e2e/orders.spec.ts`，确认新断言先失败**

### Task 2: 在共享层下沉原生列控制

**Files:**
- Create: `apps/erp-web/src/components/agGridColumnState.ts`
- Modify: `apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `apps/erp-web/src/components/ErpDataGridNext.tsx`
- Modify: `apps/erp-web/src/components/ui-modules/UiModule03MainTableSection.tsx`

- [ ] **Step 1: 抽出中文 localeText、裁剪菜单项、存储键构造与读写工具**
- [ ] **Step 2: 在 `ErpDataGrid` 接入表头右键原生菜单与中文菜单**
- [ ] **Step 3: 在 `ErpDataGrid` 接入列显隐/固定列/顺序/宽度/排序状态持久化**
- [ ] **Step 4: 让 `ErpDataGridNext`、`UiModule03MainTableSection` 透传 `columnSettingsKey` 以形成页面级持久化入口**

### Task 3: 页面接入与订单审核对齐

**Files:**
- Modify: `apps/erp-web/src/pages/orders/OrdersPage.tsx`
- Modify: `apps/erp-web/src/pages/sales/ShopOrderQueryPage.tsx`
- Modify: `apps/erp-web/src/pages/logistics/LogisticsTrackingPage.tsx`
- Modify: `apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`

- [ ] **Step 1: 为共享链路页面补稳定的 `columnSettingsKey`**
- [ ] **Step 2: 让订单审核主表复用共享列菜单裁剪与中文文案，避免双份维护**
- [ ] **Step 3: 人工检查主表框线、表头右键、列拖拽与持久化是否仍符合工业级交互**

### Task 4: 完整验证

**Files:**
- Verify only

- [ ] **Step 1: 运行 `pnpm typecheck`**
- [ ] **Step 2: 运行 `pnpm vitest run src/components/ErpDataGrid.columnState.test.tsx src/pages/orders/review-native/OrderReviewMainGrid.test.ts`**
- [ ] **Step 3: 运行 `pnpm playwright test e2e/orders.spec.ts e2e/order-review-selection.spec.ts e2e/order-review-left-panel-regression.spec.ts`**
- [ ] **Step 4: 记录结果与剩余风险，确保模板链路页面后续可复用**
