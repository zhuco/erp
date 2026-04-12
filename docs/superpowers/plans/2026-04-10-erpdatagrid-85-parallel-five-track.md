# ErpDataGrid 体感 85%+ 并行冲刺（五人）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不更换技术栈前提下，将 `ErpDataGrid` 的业务体感拉升到 85%+（相对 AG Grid 企业版核心分析链路）。

**Architecture:** 采用“五轨并行 + 单入口集成”方案。前四轨分别补齐筛选、导出、服务端行模型、图表与状态栏；第五轨专门负责入口集成与工具面板编排，避免多人同时改 `ErpDataGrid.tsx` 造成冲突。所有轨道必须以测试先行（TDD）和小步提交推进。

**Tech Stack:** React 18, TypeScript, Tabulator, Ant Design, Vitest, Playwright

---

## 并行边界（先锁定）

| Part | 负责人 | 独占写入范围（必须遵守） |
|---|---|---|
| Part 1 | A（高级筛选） | `apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx`、`.../hooks/useGridAdvancedFilter.ts`、`.../utils/advancedFilter.ts`、对应测试 |
| Part 2 | B（导出） | `apps/erp-web/src/components/erp-data-grid/utils/export.ts`、`.../utils/export.test.ts`、新增导出 E2E |
| Part 3 | C（SSRM） | `apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts`、`.../utils/serverSide.ts`、`.../utils/serverSide.test.ts`、新增 SSRM E2E |
| Part 4 | D（图表与状态栏） | `apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx`、`.../components/StatusBar.tsx`、`.../utils/chart.ts`、`.../utils/summary.ts`、对应测试 |
| Part 5 | E（集成与工具面板） | `apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx`、`.../hooks/useGridSidePanel.ts`、`docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md`、必要集成 E2E |

冲突处理规则：
1. 除 Part 5 外，其他人禁止改 `ErpDataGrid.tsx`。
2. `utils/enterpriseTypes.ts` 本轮冻结，若必须扩展，由 Part 5 统一收口。
3. 每个 Part 的 E2E 文件独立新建，避免多人改同一 spec。

---

### Task 1: Part 1 - 高级筛选器体感升级（A）

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-advanced-filter.spec.ts`

- [ ] **Step 1: 先补失败测试（筛选“可用性 + 一致性”）**
目标新增断言：
1. 嵌套组（AND/OR）可添加、删除、修改且序列化稳定。
2. `between` 单侧边界、日期字符串、数字字符串比较语义一致。
3. `blank/notBlank` 对 `null/undefined/''/[]` 行为稳定。
4. 本地 `buildFilterFunction` 结果与远端 `serializeFilterModel` 结构语义一致。

- [ ] **Step 2: 跑测试确认 FAIL**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
Expected: FAIL（新增断言至少 1 条失败）。

- [ ] **Step 3: 最小实现（不改入口）**
实现要求：
1. `AdvancedFilterPanel` 支持更顺手的组级操作（新增分组、删除分组、逻辑切换时保持值）。
2. `advancedFilter.ts` 提升比较稳定性（日期/数字归一化、between 单侧边界）。
3. `useGridAdvancedFilter.ts` 保证 `config.model` 外部受控与内部编辑不互相污染。

- [ ] **Step 4: 回归测试**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts`
Run:
`pnpm --filter erp-web exec playwright test e2e/erp-data-grid-advanced-filter.spec.ts --project=chromium`
Expected: PASS。

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts apps/erp-web/e2e/erp-data-grid-advanced-filter.spec.ts
git commit -m "feat: improve advanced filter usability and local-remote consistency"
```

---

### Task 2: Part 2 - 导出能力企业化（B）

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.test.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-export-enterprise.spec.ts`
- Modify: `F:/erp/apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md`（如需补充导出耗时口径）

- [ ] **Step 1: 先补失败测试（导出深度）**
目标新增断言：
1. `excel` 模式优先走真正 Excel 导出，不因环境判定误降级。
2. `columns='visible'`、active rows、排序/筛选上下文导出一致。
3. server mode (`serverRequest`) 取消、失败、重试路径审计事件完整。

- [ ] **Step 2: 跑测试确认 FAIL**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/export.test.ts`
Expected: FAIL。

- [ ] **Step 3: 最小实现**
实现要求：
1. 优化 `export.ts` 的 Excel 分支，减少无意义 `fallback-csv` 触发。
2. 保持“当前视图优先”导出原则（active data + visible columns）。
3. 强化审计字段：`requestId/status/mode/usedType/rowCount/duration/auditMeta`。

- [ ] **Step 4: 回归测试**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/export.test.ts`
Run:
`pnpm --filter erp-web exec playwright test e2e/erp-data-grid-export-enterprise.spec.ts --project=chromium`
Expected: PASS。

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/utils/export.ts apps/erp-web/src/components/erp-data-grid/utils/export.test.ts apps/erp-web/e2e/erp-data-grid-export-enterprise.spec.ts apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md
git commit -m "feat: harden export pipeline with enterprise-grade consistency and audit"
```

---

### Task 3: Part 3 - 服务端行模型稳定性与性能闭环（C）

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-server-side-resilience.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/erp-data-grid-performance-tier.spec.ts`（仅本人可改此文件）

- [ ] **Step 1: 先补失败测试（参数契约 + 缓存失效 + 异常恢复）**
目标新增断言：
1. sort/filter/advancedFilter/group/pivot 参数变更触发正确 refresh/hash 失效。
2. stale request 不覆盖最新数据。
3. 网络异常、Abort、重试时 loading/error 状态闭环。

- [ ] **Step 2: 跑测试确认 FAIL**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts`
Expected: FAIL。

- [ ] **Step 3: 最小实现**
实现要求：
1. `useGridServerSide` 增强块加载去重、请求版本隔离、错误恢复。
2. `serverSide.ts` 强化参数校验与稳定序列化。
3. 分档性能脚本保持 `10k/50k/100k` 三档阈值统一口径。

- [ ] **Step 4: 回归测试**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts`
Run:
`pnpm --filter erp-web exec playwright test e2e/erp-data-grid-server-side-resilience.spec.ts --project=chromium`
Run:
`pnpm --filter erp-web run test:e2e:grid-performance-tier`
Expected: PASS。

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts apps/erp-web/e2e/erp-data-grid-server-side-resilience.spec.ts apps/erp-web/e2e/erp-data-grid-performance-tier.spec.ts
git commit -m "feat: improve server-side row model resilience and tier performance stability"
```

---

### Task 4: Part 4 - 图表与状态栏分析体感提升（D）

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.test.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/StatusBar.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.test.ts`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-chart-status.spec.ts`

- [ ] **Step 1: 先补失败测试（图表/状态栏一致性）**
目标新增断言：
1. 图表数据点与当前 active view 行集一致（筛选后同步）。
2. 图表 topN + 其他桶聚合稳定，空数据提示稳定。
3. 状态栏在空集/null/负数/小数下展示和精度一致。

- [ ] **Step 2: 跑测试确认 FAIL**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/chart.test.ts src/components/erp-data-grid/utils/summary.test.ts`
Expected: FAIL。

- [ ] **Step 3: 最小实现**
实现要求：
1. `chart.ts` 保证 label/value 推断和分桶规则稳定。
2. `ChartPanel.tsx` 改善读数、图例、空态和切换体验。
3. `StatusBar.tsx` 与 `summary.ts` 保持聚合口径与显示一致。

- [ ] **Step 4: 回归测试**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/chart.test.ts src/components/erp-data-grid/utils/summary.test.ts`
Run:
`pnpm --filter erp-web exec playwright test e2e/erp-data-grid-chart-status.spec.ts --project=chromium`
Expected: PASS。

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx apps/erp-web/src/components/erp-data-grid/utils/chart.ts apps/erp-web/src/components/erp-data-grid/utils/chart.test.ts apps/erp-web/src/components/erp-data-grid/components/StatusBar.tsx apps/erp-web/src/components/erp-data-grid/utils/summary.ts apps/erp-web/src/components/erp-data-grid/utils/summary.test.ts apps/erp-web/e2e/erp-data-grid-chart-status.spec.ts
git commit -m "feat: improve chart and status bar consistency for analysis workflows"
```

---

### Task 5: Part 5 - 入口集成与工具面板统一体验（E）

**Files:**
- Modify: `F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts`
- Modify: `F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
- Modify: `F:/erp/docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md`
- Create: `F:/erp/apps/erp-web/e2e/erp-data-grid-workbench-integration.spec.ts`

- [ ] **Step 1: 先补失败测试（入口一致性）**
目标新增断言：
1. 工具面板 tab 可用性与配置一致（columns/filters/pivot/export/chart）。
2. 快捷键 `Ctrl+Shift+L/F` 行为稳定且不误触输入框。
3. 顶部动作区（导出/工具面板）在不同模式下行为一致。

- [ ] **Step 2: 跑测试确认 FAIL**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
Expected: FAIL。

- [ ] **Step 3: 最小实现（统一收口）**
实现要求：
1. `ErpDataGrid.tsx` 统一编排各 Part 新能力，保持向后兼容。
2. `SidePanel.tsx` 明确核心 tab 与自定义 tab 行为。
3. `useGridSidePanel.ts` 完善持久化和快捷键处理。
4. 更新发布闸门文档，补充 85% 体感冲刺验收证据项。

- [ ] **Step 4: 全链路回归**
Run:
`pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridSidePanel.test.ts`
Run:
`pnpm --filter erp-web exec playwright test e2e/erp-data-grid-workbench-integration.spec.ts --project=chromium`
Run:
`pnpm --filter erp-web run test:gate:grid-analysis`
Expected: PASS。

- [ ] **Step 5: 提交**
```bash
git add apps/erp-web/src/components/ErpDataGrid.tsx apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md apps/erp-web/e2e/erp-data-grid-workbench-integration.spec.ts
git commit -m "feat: integrate five-track analysis upgrades into unified ErpDataGrid workbench"
```

---

## 集成顺序与最终验收

建议合并顺序：
1. Part 1（筛选）
2. Part 3（SSRM）
3. Part 2（导出）
4. Part 4（图表与状态栏）
5. Part 5（入口集成与总门禁）

最终统一验收命令：
1. `pnpm --filter erp-web run typecheck`
2. `pnpm --filter erp-web run test:unit:grid-analysis`
3. `pnpm --filter erp-web run test:e2e:grid-analysis`
4. `pnpm --filter erp-web run test:e2e:grid-performance-tier`
5. `pnpm --filter erp-web run test:gate:grid-analysis`

体感 85%+ 判定标准（发布前必须全部满足）：
1. 分析链路（筛选/透视/导出/图表/服务端行模型）E2E 全通过。
2. 分档性能（10k/50k/100k）全部达标。
3. 导出一致性与审计链路可复现。
4. 工具面板与快捷键交互无阻断缺陷。
5. 发布闸门文档证据项可归档。

