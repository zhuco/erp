# ErpDataGrid 对标 AG Grid 企业版全量能力（多 AI 协同）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 以 AG Grid 企业版全量能力为基线，在现有 `ErpDataGrid` 上完成可发布的能力补齐与门禁闭环。

**Architecture:** 按“能力域解耦 + 单入口集成”执行。8 个并行 AI 分别负责过滤、分组透视、SSRM、导出、图表、编辑交互、树/明细、入口集成与发布门禁。除集成轨外，禁止修改 `ErpDataGrid.tsx`，通过独占写入边界避免冲突。

**Tech Stack:** React 18, TypeScript, Tabulator, Ant Design, Vitest, Playwright, pnpm

---

## 1. 对标基线（AG Grid 企业版全量能力）

对标范围采用 AG Grid 官方“Enterprise Features + Modules”口径，覆盖但不限于：

1. 服务端行模型（Server-Side Row Model）
2. 分组/聚合/透视（Row Grouping, Aggregation, Pivoting）
3. 树形/主从明细（Tree Data, Master Detail）
4. 高级过滤（Advanced / Set / Multi Filter）
5. 侧边栏与工具面板（Side Bar, Columns/Filters/Row Group Panel）
6. 菜单体系（Column Menu, Context Menu）
7. 区域选择与填充（Range Selection, Fill Handle）
8. 剪贴板与编辑历史（Clipboard, Undo/Redo Editing）
9. Excel 企业导出能力（样式、公式、多 Sheet）
10. 集成图表与可视化（Integrated Charts, Sparklines）
11. 状态栏与汇总视图（Status Bar / Summary）
12. 性能与发布门禁（性能分档、稳定性、类型安全、CI 证据）

官方口径参考：
- Modules（含 Enterprise Features 与模块列表）：https://www.ag-grid.com/javascript-data-grid/modules/
- Community vs Enterprise（能力对比）：https://www.ag-grid.com/javascript-data-grid/community-vs-enterprise/

---

## 2. 当前差距量化（全量能力视角）

### 2.1 能力矩阵（当前仓库）

| 领域 | 当前状态 | 结论 |
|---|---|---|
| 高级筛选 | 已有 Advanced Filter 主链路与测试 | **部分达标**（Set/Multi/Floating 行为仍需增强） |
| 分组/聚合/透视 | 已有 `useGridGrouping` / `useGridPivot` / `aggregation` | **部分达标**（企业级面板联动与服务端一致性仍需补） |
| SSRM | 已有 `useGridServerSide` 与性能档位测试 | **部分达标**（恢复韧性仍有失败用例） |
| 导出 | 已有 CSV/Excel 与 serverRequest 链路 | **部分达标**（Excel 样式/公式/多 Sheet 仍有差距） |
| 图表/状态栏 | 已有 `ChartPanel` 与 `StatusBar` | **部分达标**（交互深度与图表能力层级不足） |
| 工具面板与快捷键 | 已有 SidePanel + 快捷键 + 集成测试 | **部分达标**（全量面板行为一致性仍需强化） |
| 树形/明细 | 已有 `useGridTree`/`useGridDetail` | **部分达标**（层级选择/复杂场景一致性待补） |
| 菜单体系 | 已有 context menu 能力入口 | **部分达标**（列菜单能力和企业级菜单扩展不足） |
| 编辑/区域操作 | 已有 `useGridEditing`、clipboard、range copy | **部分达标**（Fill Handle、批量编辑一致性仍需补） |
| 发布门禁 | unit/e2e/perf 可跑，`test:gate:grid-analysis` 仍失败 | **未达标**（类型检查未清零） |

### 2.2 当前体感估算（全量能力口径）

- 当前估算：**60% ~ 68%**
- 与“AG Grid 企业版全量能力体感 85%+”差距：**17 ~ 25 个点**
- 关键阻断：
1. `typecheck` 未通过，导致总门禁不可放行
2. SSRM 快速切换恢复场景存在稳定失败
3. Enterprise 深水区能力（Excel 深度、图表深度、Fill Handle、菜单体系）仍不足

---

## 3. 并行协同边界（8 AI）

## 3.1 总规则

1. **仅 AI-H（集成）可修改** `apps/erp-web/src/components/ErpDataGrid.tsx`
2. 每个 AI 只改自己白名单文件，跨域需求通过 AI-H 收口
3. 每个 AI 必须先补失败测试，再实现，再回归
4. 每个 AI 独立 worktree/分支开发，最终由 AI-H 统一合并

## 3.2 任务分工

| AI | 领域 | 独占写入范围（必须遵守） |
|---|---|---|
| AI-A | 过滤器企业化 | `components/AdvancedFilterPanel.tsx`、`hooks/useGridAdvancedFilter.ts`、`utils/advancedFilter.ts`、对应测试、`e2e/erp-data-grid-advanced-filter.spec.ts` |
| AI-B | 分组/透视/聚合 | `components/PivotPanel.tsx`、`hooks/useGridGrouping.ts`、`hooks/useGridPivot.ts`、`utils/pivot.ts`、`utils/aggregation.ts`、对应测试、`e2e/erp-data-grid-group-pivot-enterprise.spec.ts` |
| AI-C | SSRM 韧性与性能 | `hooks/useGridServerSide.ts`、`utils/serverSide.ts`、对应测试、`e2e/erp-data-grid-server-side-resilience.spec.ts`、`e2e/erp-data-grid-performance-tier.spec.ts` |
| AI-D | 导出企业化 | `utils/export.ts`、`utils/export.test.ts`、`e2e/erp-data-grid-export-enterprise.spec.ts`、`apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md` |
| AI-E | 图表与状态栏深度 | `components/ChartPanel.tsx`、`components/StatusBar.tsx`、`utils/chart.ts`、`utils/summary.ts`、对应测试、`e2e/erp-data-grid-chart-status.spec.ts` |
| AI-F | 编辑/区域/剪贴板 | `hooks/useGridEditing.tsx`、`hooks/useGridKeyboard.ts`、`utils/clipboard.ts`、`erp-data-grid/ErpDataGridImpl.tsx`、新增对应测试与 `e2e/erp-data-grid-range-editing.spec.ts` |
| AI-G | 树形/明细/层级一致性 | `hooks/useGridTree.ts`、`hooks/useGridDetail.ts`、对应测试、`e2e/erp-data-grid-tree-detail-enterprise.spec.ts` |
| AI-H | 入口集成与发布门禁 | `components/ErpDataGrid.tsx`、`components/erp-data-grid/components/SidePanel.tsx`、`hooks/useGridSidePanel.ts`、`hooks/useGridSidePanel.test.ts`、`docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md`、`e2e/erp-data-grid-workbench-integration.spec.ts`、门禁脚本相关文档 |

---

## 4. 执行命令（可直接复制）

## 4.1 基础并行命令（Tech Lead 执行一次）

```bash
git fetch origin
git worktree add ../erp-ai-a -b feat/grid-full-a origin/main
git worktree add ../erp-ai-b -b feat/grid-full-b origin/main
git worktree add ../erp-ai-c -b feat/grid-full-c origin/main
git worktree add ../erp-ai-d -b feat/grid-full-d origin/main
git worktree add ../erp-ai-e -b feat/grid-full-e origin/main
git worktree add ../erp-ai-f -b feat/grid-full-f origin/main
git worktree add ../erp-ai-g -b feat/grid-full-g origin/main
git worktree add ../erp-ai-h -b feat/grid-full-h origin/main
```

## 4.2 AI-A（过滤器企业化）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-advanced-filter.spec.ts --project=chromium
git add apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts apps/erp-web/e2e/erp-data-grid-advanced-filter.spec.ts
git commit -m "feat: harden enterprise filter semantics and panel consistency"
```

## 4.3 AI-B（分组/透视/聚合）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/pivot.test.ts src/components/erp-data-grid/utils/aggregation.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-group-pivot-enterprise.spec.ts --project=chromium
git add apps/erp-web/src/components/erp-data-grid/components/PivotPanel.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridGrouping.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridPivot.ts apps/erp-web/src/components/erp-data-grid/utils/pivot.ts apps/erp-web/src/components/erp-data-grid/utils/pivot.test.ts apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts apps/erp-web/src/components/erp-data-grid/utils/aggregation.test.ts apps/erp-web/e2e/erp-data-grid-group-pivot-enterprise.spec.ts
git commit -m "feat: improve enterprise grouping pivot and aggregation parity"
```

## 4.4 AI-C（SSRM 韧性与性能）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-server-side-resilience.spec.ts --project=chromium
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-performance-tier.spec.ts --project=chromium --workers=2
git add apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts apps/erp-web/e2e/erp-data-grid-server-side-resilience.spec.ts apps/erp-web/e2e/erp-data-grid-performance-tier.spec.ts
git commit -m "feat: close ssrm resilience and tier-performance gaps"
```

## 4.5 AI-D（导出企业化）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/export.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-export-enterprise.spec.ts --project=chromium
git add apps/erp-web/src/components/erp-data-grid/utils/export.ts apps/erp-web/src/components/erp-data-grid/utils/export.test.ts apps/erp-web/e2e/erp-data-grid-export-enterprise.spec.ts apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md
git commit -m "feat: upgrade enterprise export depth with stable audit chain"
```

## 4.6 AI-E（图表与状态栏）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/chart.test.ts src/components/erp-data-grid/utils/summary.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-chart-status.spec.ts --project=chromium
git add apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx apps/erp-web/src/components/erp-data-grid/components/StatusBar.tsx apps/erp-web/src/components/erp-data-grid/utils/chart.ts apps/erp-web/src/components/erp-data-grid/utils/chart.test.ts apps/erp-web/src/components/erp-data-grid/utils/summary.ts apps/erp-web/src/components/erp-data-grid/utils/summary.test.ts apps/erp-web/e2e/erp-data-grid-chart-status.spec.ts
git commit -m "feat: improve chart and status-bar enterprise consistency"
```

## 4.7 AI-F（编辑/区域/剪贴板）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridKeyboard.test.ts src/components/erp-data-grid/hooks/useGridEditing.test.ts src/components/erp-data-grid/utils/clipboard.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-range-editing.spec.ts --project=chromium
git add apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.ts apps/erp-web/src/components/erp-data-grid/utils/clipboard.ts apps/erp-web/src/components/erp-data-grid/ErpDataGridImpl.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.test.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.test.ts apps/erp-web/src/components/erp-data-grid/utils/clipboard.test.ts apps/erp-web/e2e/erp-data-grid-range-editing.spec.ts
git commit -m "feat: align range clipboard and undo-redo interactions with enterprise behavior"
```

## 4.8 AI-G（树形/明细/层级一致性）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridTree.test.ts src/components/erp-data-grid/hooks/useGridDetail.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-tree-detail-enterprise.spec.ts --project=chromium
git add apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.test.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.test.ts apps/erp-web/e2e/erp-data-grid-tree-detail-enterprise.spec.ts
git commit -m "feat: harden tree-detail hierarchy consistency for enterprise workflows"
```

## 4.9 AI-H（入口集成与总门禁）

```bash
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridSidePanel.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-workbench-integration.spec.ts --project=chromium
pnpm --filter erp-web run typecheck
pnpm --filter erp-web run test:unit:grid-analysis
pnpm --filter erp-web run test:e2e:grid-analysis
pnpm --filter erp-web run test:e2e:grid-performance-tier
pnpm --filter erp-web run test:gate:grid-analysis
git add apps/erp-web/src/components/ErpDataGrid.tsx apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md apps/erp-web/e2e/erp-data-grid-workbench-integration.spec.ts
git commit -m "feat: integrate full enterprise-parity tracks into ErpDataGrid and close release gates"
```

---

## 5. 合并顺序与收口标准

## 5.1 推荐合并顺序

1. AI-A（过滤）
2. AI-B（分组/透视/聚合）
3. AI-D（导出）
4. AI-E（图表/状态栏）
5. AI-F（编辑/区域/剪贴板）
6. AI-G（树/明细）
7. AI-C（SSRM 韧性与性能）
8. AI-H（入口集成与总门禁）

## 5.2 发布前必须满足（85%+ 体感）

1. `pnpm --filter erp-web run test:gate:grid-analysis` 一次通过
2. SSRM 韧性用例无失败，且 `10k/50k/100k` 分档性能持续达标
3. 过滤/透视/导出/图表/工具面板 E2E 全通过
4. 关键交互链路（快捷键、菜单、剪贴板、编辑撤销）无阻断缺陷
5. `docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md` 证据项可归档
