# ErpDataGrid 95% 企业能力对齐（六 AI 并行）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不替换现有引擎（Tabulator + ErpDataGrid）的前提下，将组件对齐 AG Grid 企业版核心能力体感提升到 95%，并具备工业级可发布门禁。

**Architecture:** 采用“六轨并行 + 单入口集成 + 强门禁”模式。5 个能力轨负责独占模块开发，1 个集成轨负责入口编排、冲突收敛、门禁和发布证据。除集成轨外，禁止修改 `ErpDataGrid.tsx`。

**Tech Stack:** React 18, TypeScript, Tabulator, Ant Design, Vitest, Playwright, pnpm

---

## 0. 执行总则（必须遵守）

1. 单入口原则：只有 `AI-6` 可改 `apps/erp-web/src/components/ErpDataGrid.tsx`。
2. 独占写入原则：每个 AI 只改本轨白名单文件，不跨轨改文件。
3. TDD 原则：先补失败测试，再实现，再回归，再提交。
4. 小步提交：每个里程碑至少 1 次可回滚 commit。
5. 验证优先：未通过指定命令，不允许标记“完成”。
6. 工业级约束：必须补齐审计、可观测、异常恢复、权限边界。

---

## 1. 95% 目标能力清单（交付范围）

### 1.1 必达能力域

- [ ] SSRM：分块加载、缓存驱逐、重试、取消、参数契约、summary 合并。
- [ ] 过滤：Advanced Filter、Set Filter、Multi Filter、Floating Filter、本地远程一致性。
- [ ] 分组聚合透视：row group、pivot、小计、总计、服务端模式参数联动。
- [ ] 编辑交互：范围选择、Fill Handle、Clipboard Copy/Paste、Undo/Redo、批量编辑。
- [ ] 树形与明细：Tree Data、懒加载、Master/Detail、状态持久化。
- [ ] 导出与打印：CSV/Excel、服务端导出、取消重试、导出审计、打印一致性。
- [ ] 图表与状态栏：bar/line/pie、pivot 图表一致性、sparklines、summary 精度稳定。
- [ ] 菜单与面板：Column Menu、Context Menu、SidePanel tabs、自定义面板、快捷键。
- [ ] 治理与发布：性能分档门禁、并发稳定性、权限与审计、发布证据归档。

### 1.2 可暂缓能力（归入剩余 5%）

- [ ] AG Grid 同名 API 的逐项 1:1 参数兼容。
- [ ] 高级图表工作台级配置器（超当前业务需求）。

---

## 2. 六 AI 并行分工与文件边界

## 2.1 AI-1 过滤体系企业化（Set/Multi/Floating）

**Owner:** AI-1  
**职责:** 完成过滤能力深度补齐与本地远程语义对齐。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/FilterBar.tsx`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/SetFilterPanel.tsx`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/MultiFilterPanel.tsx`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/FloatingFilterRow.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-enterprise-filter-suite.spec.ts`

**任务步骤:**

- [ ] Step 1: 增加失败测试（Set/Multi/Floating 与 Advanced 组合语义）。
- [ ] Step 2: 运行 `vitest` 定位失败并固化断言。
- [ ] Step 3: 实现 Set Filter（去重值、搜索、全选、反选）。
- [ ] Step 4: 实现 Multi Filter（同列组合过滤器）。
- [ ] Step 5: 实现 Floating Filter Row（列头快速过滤输入）。
- [ ] Step 6: 打通本地谓词与远端序列化语义一致。
- [ ] Step 7: 跑单测 + E2E 回归并提交。

**验证命令:**

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-enterprise-filter-suite.spec.ts --project=chromium
```

---

## 2.2 AI-2 SSRM + 性能 + 契约韧性

**Owner:** AI-2  
**职责:** 完成服务端行模型工业化能力与分档性能稳态。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-server-side-resilience.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-performance-tier.spec.ts`
- Modify: `F:/erp/apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md`

**任务步骤:**

- [ ] Step 1: 增加失败测试（参数契约、取消、重试、stale 请求隔离）。
- [ ] Step 2: 强化 `BlockCache` 驱逐与 materialized rows 上限行为。
- [ ] Step 3: 强化请求版本与取消链路，避免旧响应污染新状态。
- [ ] Step 4: 强化异常恢复（超时/Abort/5xx）与 loading/error 一致性。
- [ ] Step 5: 对齐 10k/50k/100k 阈值，更新性能基线文档。
- [ ] Step 6: 跑单测 + E2E + 性能分档并提交。

**验证命令:**

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-server-side-resilience.spec.ts --project=chromium
pnpm --filter erp-web run test:e2e:grid-performance-tier
```

---

## 2.3 AI-3 分组/透视/树形/明细

**Owner:** AI-3  
**职责:** 完成层级分析能力一致性（group/pivot/tree/detail）。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridGrouping.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridPivot.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/PivotPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.test.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-group-pivot-enterprise.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-tree-detail-enterprise.spec.ts`

**任务步骤:**

- [ ] Step 1: 增加失败测试（小计总计一致、层级恢复一致、懒加载一致）。
- [ ] Step 2: 完善 pivot subtotal/grandtotal 边界行为与稳定排序。
- [ ] Step 3: 完善 group 展开状态持久化与恢复。
- [ ] Step 4: 完善 tree + detail 与筛选/排序场景的一致性。
- [ ] Step 5: 完成联动 E2E 并提交。

**验证命令:**

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/pivot.test.ts src/components/erp-data-grid/utils/aggregation.test.ts src/components/erp-data-grid/hooks/useGridTree.test.ts src/components/erp-data-grid/hooks/useGridDetail.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-group-pivot-enterprise.spec.ts e2e/erp-data-grid-tree-detail-enterprise.spec.ts --project=chromium
```

---

## 2.4 AI-4 编辑/范围/剪贴板工业化（含 Fill Handle、Paste）

**Owner:** AI-4  
**职责:** 完成企业编辑深水区：Fill Handle、Clipboard Paste、历史一致性。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/clipboard.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/ErpDataGridImpl.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/clipboard.test.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-range-editing.spec.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-fill-paste-enterprise.spec.ts`

**任务步骤:**

- [ ] Step 1: 新增失败测试（Fill Handle、Paste、Undo/Redo 栈一致性）。
- [ ] Step 2: 实现 Clipboard Paste（TSV/CSV 解析、范围写入、校验失败处理）。
- [ ] Step 3: 实现 Fill Handle（向下/向右填充，类型感知）。
- [ ] Step 4: 对齐键盘行为（copy/paste/edit/cancel/navigation）。
- [ ] Step 5: 保证批量编辑与历史栈一致性可回放。
- [ ] Step 6: 跑单测 + E2E 并提交。

**验证命令:**

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridEditing.test.ts src/components/erp-data-grid/hooks/useGridKeyboard.test.ts src/components/erp-data-grid/utils/clipboard.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-range-editing.spec.ts e2e/erp-data-grid-fill-paste-enterprise.spec.ts --project=chromium
```

---

## 2.5 AI-5 导出/图表/状态栏深度企业化

**Owner:** AI-5  
**职责:** 完成导出深度、图表深度、sparklines 与状态栏一致性。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/StatusBar.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.test.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-export-enterprise.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-chart-status.spec.ts`
- Create: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/SparklineCell.tsx`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-sparkline.spec.ts`

**任务步骤:**

- [ ] Step 1: 新增失败测试（Excel 样式/多 Sheet/公式；图表与当前视图一致）。
- [ ] Step 2: 强化 export server/client 两模式，补全审计链状态。
- [ ] Step 3: 增加 Sparkline 单元格组件与渲染回退策略。
- [ ] Step 4: 强化 summary 精度边界（-0、小数、空集）一致显示。
- [ ] Step 5: 跑单测 + E2E 并提交。

**验证命令:**

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/export.test.ts src/components/erp-data-grid/utils/chart.test.ts src/components/erp-data-grid/utils/summary.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-export-enterprise.spec.ts e2e/erp-data-grid-chart-status.spec.ts e2e/erp-data-grid-sparkline.spec.ts --project=chromium
```

---

## 2.6 AI-6 入口集成/菜单面板/治理门禁（总装轨）

**Owner:** AI-6（集成负责人）  
**职责:** 统一收口所有能力、完成门禁纳管、发布证据归档。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/ColumnSettingsPanel.tsx`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-workbench-integration.spec.ts`
- Modify: `F:/erp/apps/erp-web/package.json`
- Modify: `F:/erp/docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-enterprise-gate.spec.ts`
- Create: `F:/erp/docs/web/ERPDATAGRID_ENTERPRISE_95_RELEASE_CHECKLIST.md`

**任务步骤:**

- [ ] Step 1: 接入 AI-1~AI-5 能力开关与入口编排，不破坏兼容行为。
- [ ] Step 2: 完善 Column Menu/Context Menu 行为模型与权限钩子。
- [ ] Step 3: 统一 SidePanel tab 与 customTabs 生命周期。
- [ ] Step 4: 将企业专项 E2E 纳入 `test:gate:grid-analysis` 或新增 `test:gate:grid-enterprise-95`。
- [ ] Step 5: 追加可观测与审计字段校验（导出/批量编辑/关键交互）。
- [ ] Step 6: 更新发布闸门文档和证据模板。
- [ ] Step 7: 执行全量门禁并提交。

**验证命令:**

```bash
pnpm --filter erp-web run typecheck
pnpm --filter erp-web run test:unit:grid-analysis
pnpm --filter erp-web run test:e2e:grid-analysis
pnpm --filter erp-web run test:e2e:grid-performance-tier
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-enterprise-filter-suite.spec.ts e2e/erp-data-grid-group-pivot-enterprise.spec.ts e2e/erp-data-grid-export-enterprise.spec.ts e2e/erp-data-grid-chart-status.spec.ts e2e/erp-data-grid-range-editing.spec.ts e2e/erp-data-grid-fill-paste-enterprise.spec.ts e2e/erp-data-grid-tree-detail-enterprise.spec.ts e2e/erp-data-grid-sparkline.spec.ts e2e/erp-data-grid-workbench-integration.spec.ts e2e/erp-data-grid-enterprise-gate.spec.ts --project=chromium
pnpm --filter erp-web run test:gate:grid-analysis
```

---

## 3. 并行启动与分支命令

```bash
git fetch origin
git worktree add ../erp-ai-1 -b feat/grid95-ai1 origin/main
git worktree add ../erp-ai-2 -b feat/grid95-ai2 origin/main
git worktree add ../erp-ai-3 -b feat/grid95-ai3 origin/main
git worktree add ../erp-ai-4 -b feat/grid95-ai4 origin/main
git worktree add ../erp-ai-5 -b feat/grid95-ai5 origin/main
git worktree add ../erp-ai-6 -b feat/grid95-ai6 origin/main
```

---

## 4. 合并顺序（减少冲突）

1. AI-1（过滤）
2. AI-3（分组/透视/树明细）
3. AI-4（编辑/范围/剪贴板）
4. AI-5（导出/图表/状态栏）
5. AI-2（SSRM/性能）
6. AI-6（集成/门禁/文档）

---

## 5. 95% 发布 DoD（Definition of Done）

### 5.1 功能 DoD

- [ ] Set/Multi/Floating Filter 均可用，且与 Advanced Filter 无冲突。
- [ ] Fill Handle 与 Clipboard Paste 在范围选择模式稳定可用。
- [ ] Pivot subtotal/grandtotal 与原始数据数学一致。
- [ ] Tree/Detail 状态恢复在刷新/筛选/分页后一致。
- [ ] 导出审计链（started/success/failed/cancelled）可追溯。
- [ ] Sparklines 可在网格内稳定渲染并提供降级路径。

### 5.2 质量 DoD

- [ ] `pnpm --filter erp-web run typecheck` 通过。
- [ ] `pnpm --filter erp-web run test:unit:grid-analysis` 通过。
- [ ] `pnpm --filter erp-web run test:e2e:grid-analysis` 通过。
- [ ] `pnpm --filter erp-web run test:e2e:grid-performance-tier` 通过。
- [ ] 企业专项 E2E 套件全通过（见 AI-6 验证命令）。
- [ ] `pnpm --filter erp-web run test:gate:grid-analysis` 通过。

### 5.3 工业化 DoD

- [ ] 关键操作埋点完整（过滤、透视、导出、编辑、图表）。
- [ ] 权限与租户隔离检查通过（缓存 key、审计字段、导出权限）。
- [ ] 发布证据归档完成（命令、结果、版本、时间、责任人）。

---

## 6. 风险与止损策略

- [ ] 若 AI-4（Fill/Paste）延期：先放行 copy + undo/redo 稳态，paste/fill 以 feature flag 灰度。
- [ ] 若 AI-5（Excel 深度）延期：保持 server export + 审计闭环，样式/公式/多 Sheet 设为后续补丁。
- [ ] 若 AI-2（性能）异常：冻结新特性，优先修复性能分档门禁。
- [ ] 若集成冲突超 2 次：由 AI-6 统一 rebase + cherry-pick，禁止双向改入口文件。

---

## 7. 最终交付物清单

- [ ] 代码实现（6 条分支合并）。
- [ ] 新增/更新测试（unit + e2e + gate）。
- [ ] 更新发布门禁文档：
  - `F:/erp/docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md`
  - `F:/erp/docs/web/ERPDATAGRID_ENTERPRISE_95_RELEASE_CHECKLIST.md`
- [ ] 发布证据包（命令日志、测试结果、性能阈值、审计样例）。

