# ErpDataGrid 对标 AG Grid 企业版（分析能力）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 以工业级标准补齐 `ErpDataGrid` 分析能力，对标 AG Grid 企业版核心分析链路，并建立可持续测试与发布门禁。

**Architecture:** 保留 `ErpDataGrid` 产品层 API，优先补齐“能力已实现但未接入主链路”的断点（`side panel`、`server-side`、`tree/detail`、`chart`），并修复高级筛选/透视能力缺口。测试分层采用“纯函数单测 -> Hook/契约单测 -> 页面级 E2E -> 性能基线门禁”。

**Tech Stack:** React 18, TypeScript, Tabulator, Ant Design, Vitest, Playwright

---

## 0. 现状快照（作为排期基线）

- 已接入主链路能力：
  - 多列排序、分组、聚合、高级筛选、本地透视、导出、状态栏、快捷键与剪贴板、批量编辑（含 undo/redo）
- 已有实现但未接入主链路：
  - `useGridSidePanel` / `SidePanel`
  - `useGridServerSide`
  - `useGridTree`
  - `useGridDetail`
  - `ChartPanel`
- 测试现状：
  - `erp-data-grid-next/core` 有单测
  - 旧 `erp-data-grid` 分析域基本无自动化覆盖

---

## 1. 文件结构与责任边界

### 1.1 主组件接入层
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/types.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/index.ts`

责任说明：
- 在不破坏兼容的前提下，把企业版能力配置真正串到主链路。

### 1.2 分析能力模块
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridPivot.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts`

责任说明：
- 修复透视/筛选能力缺口。
- 完成服务端参数契约、缓存和滚动加载行为闭环。

### 1.3 面板与页面体验
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/PivotPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrdersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`

责任说明：
- 将分析能力在真实业务页面落地，确保交互可被用户直接使用。

### 1.4 自动化测试
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-analysis.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/orders.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`

---

## 2. 周排期（6 周）

### Week 1（P0）：能力接入闭环

### Task 1: 侧边工具面板接入主链路
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`

- [ ] **Step 1: 先写失败测试（侧栏打开状态持久化 + Tab 切换）**
```ts
import { describe, expect, it } from 'vitest';
// 断言：defaultOpen/defaultTab 正确；storage 恢复正确；Ctrl+Shift+L/F 生效
```

- [ ] **Step 2: 运行测试确认失败**
Run: `pnpm exec vitest --run src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
Expected: FAIL（功能尚未接入主链路）

- [ ] **Step 3: 最小实现**
```ts
// ErpDataGrid.tsx 新增 sidePanelConfig props
// 挂载 SidePanel / SidePanelTrigger，并接入 columns/filters/pivot/export/chart tab
```

- [ ] **Step 4: 运行测试确认通过**
Run: `pnpm exec vitest --run src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/ErpDataGrid.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts
git commit -m "feat: wire side panel into ErpDataGrid main flow"
```

### Task 2: 服务端行模型接入主链路
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts`

- [ ] **Step 1: 先写失败测试（参数构建、block 计算、缓存行为）**
- [ ] **Step 2: 跑单测确认失败**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// rowModel === 'server' 时启用 useGridServerSide
// 将 sort/filter/advancedFilter/group/pivot 参数透传 getRows
```

- [ ] **Step 4: 复跑单测**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/ErpDataGrid.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts
git commit -m "feat: enable server-side row model in ErpDataGrid"
```

### Week 2（P0）：透视与高级筛选修补

### Task 3: 透视能力完善（多列维与小计）
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridPivot.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/PivotPanel.tsx`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.test.ts`

- [ ] **Step 1: 先写失败测试（多列维、showSubTotal、showGrandTotal）**
- [ ] **Step 2: 跑单测确认失败**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/pivot.test.ts`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// computePivot 支持 columnGroups 多层展开
// showSubTotal 实际输出 subtotal 行
```

- [ ] **Step 4: 复跑单测**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/pivot.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/utils/pivot.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridPivot.ts apps/erp-web/src/components/erp-data-grid/components/PivotPanel.tsx apps/erp-web/src/components/erp-data-grid/utils/pivot.test.ts
git commit -m "feat: improve pivot multi-dimension and subtotal support"
```

### Task 4: 高级筛选完善（嵌套组与算子一致性）
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts`
- Test: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`

- [ ] **Step 1: 先写失败测试（嵌套组 AND/OR、between、blank/notBlank）**
- [ ] **Step 2: 跑单测确认失败**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// Panel 支持新增子组
// buildFilterFunction 对嵌套组递归求值
```

- [ ] **Step 4: 复跑单测**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts
git commit -m "feat: support nested advanced filter groups and operator parity"
```

### Week 3（P1）：树形/主从明细与图表能力闭环

### Task 5: 树形与主从明细接入主链路
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`

- [ ] **Step 1: 先写 E2E 失败用例（展开、折叠、懒加载、详情）**
- [ ] **Step 2: 跑 E2E 确认失败**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts -g "tree and detail"`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// treeConfig/detailConfig 在 ErpDataGrid.tsx 挂接
// 展开状态按 rowKey 持久化
```

- [ ] **Step 4: 复跑 E2E**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts -g "tree and detail"`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/ErpDataGrid.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.ts apps/erp-web/src/pages/orders/OrderReviewPage.tsx
git commit -m "feat: integrate tree data and master-detail into ErpDataGrid"
```

### Task 6: 图表面板从占位升级到可用
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.ts`
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`

- [ ] **Step 1: 先写 E2E 失败用例（chart tab 打开、数据同步）**
- [ ] **Step 2: 跑 E2E 确认失败**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts -g "chart panel"`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// chartConfig.enabled 时显示 chart tab
// 与当前过滤结果、分组/透视结果同步
```

- [ ] **Step 4: 复跑 E2E**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts -g "chart panel"`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx apps/erp-web/src/components/erp-data-grid/utils/chart.ts apps/erp-web/src/components/ErpDataGrid.tsx
git commit -m "feat: make chart panel usable with analysis data flow"
```

### Week 4（P1）：导出一致性与聚合精度

### Task 7: 导出一致性（当前视图即导出视图）
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.ts`
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Test: `F:/erp/apps/erp-web/e2e/erp-data-grid-analysis.spec.ts`

- [ ] **Step 1: 先写 E2E 失败用例（过滤/排序/透视后导出）**
- [ ] **Step 2: 跑 E2E 确认失败**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts -g "export consistency"`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// 导出前统一拿 active data + visible columns + current sort/filter context
```

- [ ] **Step 4: 复跑 E2E**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts -g "export consistency"`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/utils/export.ts apps/erp-web/src/components/ErpDataGrid.tsx apps/erp-web/e2e/erp-data-grid-analysis.spec.ts
git commit -m "feat: align export output with active analysis view"
```

### Task 8: 聚合精度与状态栏一致性
**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.test.ts`

- [ ] **Step 1: 先写失败测试（sum/avg/min/max/count、precision）**
- [ ] **Step 2: 跑单测确认失败**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/aggregation.test.ts`
Expected: FAIL

- [ ] **Step 3: 最小实现**
```ts
// 统一 rounding/precision 行为
// group header 与 status bar 使用一致计算口径
```

- [ ] **Step 4: 复跑单测**
Run: `pnpm exec vitest --run src/components/erp-data-grid/utils/aggregation.test.ts`
Expected: PASS

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts apps/erp-web/src/components/erp-data-grid/utils/summary.ts apps/erp-web/src/components/erp-data-grid/utils/aggregation.test.ts
git commit -m "fix: unify aggregation precision and status bar consistency"
```

### Week 5（P2）：E2E 回归包与性能基线

### Task 9: 分析功能 E2E 回归包
**Files:**
- Create/Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-analysis.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/orders.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/order-review-selection.spec.ts`

- [ ] **Step 1: 编写分析链路场景**
1. 高级筛选（AND/OR）  
2. 透视切换与小计/总计  
3. 分组聚合  
4. 服务端行模型滚动预取  
5. 树形/详情  
6. 导出一致性

- [ ] **Step 2: 跑 E2E**
Run: `pnpm exec playwright test e2e/erp-data-grid-analysis.spec.ts`
Expected: PASS

- [ ] **Step 3: 提交**
```bash
git add apps/erp-web/e2e/erp-data-grid-analysis.spec.ts apps/erp-web/e2e/orders.spec.ts apps/erp-web/e2e/order-review-selection.spec.ts
git commit -m "test: add analysis regression e2e suite for ErpDataGrid"
```

### Task 10: 性能基线与门禁
**Files:**
- Create: `F:/erp/apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-performance.spec.ts`

- [ ] **Step 1: 建立 10k/50k/100k 数据集场景**
- [ ] **Step 2: 定义性能阈值**
1. 首屏渲染  
2. 滚动帧率  
3. 筛选耗时  
4. 透视耗时

- [ ] **Step 3: 跑性能 smoke**
Run: `pnpm exec playwright test e2e/erp-data-grid-performance.spec.ts`
Expected: PASS（不超过阈值）

- [ ] **Step 4: 提交**
```bash
git add apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md apps/erp-web/e2e/erp-data-grid-performance.spec.ts
git commit -m "test: add ErpDataGrid analysis performance baseline"
```

### Week 6（P2）：发布准备与验收封板

### Task 11: 统一验收与发布门禁
**Files:**
- Modify: `F:/erp/docs/web/TESTING.md`
- Modify: `F:/erp/apps/erp-web/package.json`

- [ ] **Step 1: 新增分析域测试命令**
```json
{
  "scripts": {
    "test:unit:grid-analysis": "vitest --run src/components/erp-data-grid/**/*.test.ts",
    "test:e2e:grid-analysis": "playwright test e2e/erp-data-grid-analysis.spec.ts"
  }
}
```

- [ ] **Step 2: 更新文档并明确发布前必跑项**
- [ ] **Step 3: 执行总验收**
Run: `pnpm run lint && pnpm run typecheck && pnpm run test:unit && pnpm run test:e2e:grid-analysis`
Expected: 全部 PASS

- [ ] **Step 4: 提交**
```bash
git add docs/web/TESTING.md apps/erp-web/package.json
git commit -m "chore: add analysis-domain quality gates for release"
```

---

## 3. 工业级验收标准（DoD）

1. 功能 DoD
- 侧栏、透视、高级筛选、服务端行模型、树形、主从明细、导出、状态栏全部可在业务页直接使用。

2. 质量 DoD
- 分析域核心单测通过率 100%。
- 分析域 E2E 回归包通过率 100%。

3. 性能 DoD
- 在约定数据量下满足性能阈值，不出现明显卡顿与交互阻塞。

4. 兼容 DoD
- 未启用新配置时，历史页面行为不变（向后兼容）。

---

## 4. 风险与应对

1. 风险：`ErpDataGrid` 与 `ErpDataGridNext` 双轨导致行为口径不一致。  
应对：分析域契约测试以旧主链为准，Next 仅作为未来迁移参考。

2. 风险：透视/高级筛选复杂组合引发性能回退。  
应对：Week 5 引入性能基线和固定数据集回归。

3. 风险：E2E 不稳定。  
应对：关键断言使用 `data-testid`，避免 brittle selector。

---

Plan complete and saved to `docs/superpowers/plans/2026-04-09-erpdatagrid-enterprise-analysis-parity.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
