# ErpDataGrid 95% 六 AI 启动指令模板

用途：给 6 个 AI 并行开发任务时，作为“首条指令”直接复制发送。  
主计划：`F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md`

---

## 通用广播模板（先发给全部 AI）

```text
你在 F:/erp 的独立 worktree 中工作。目标是按以下主计划执行：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

硬性约束：
1) 只修改你被分配的白名单文件，不跨轨改文件。
2) 先写失败测试，再实现，再回归（TDD）。
3) 不要回滚或覆盖其他 AI 的改动；如发现冲突，保留现场并说明。
4) 每个里程碑至少一次提交，提交信息用中文或英文都可，但要含模块关键词。
5) 完成后必须给出：变更文件清单、执行命令、测试结果、风险与后续建议。

输出要求：
- 每完成一个步骤，给出“步骤编号 + 结果”。
- 最终输出严格包含 4 段：
  A. Changed Files
  B. Commands Run
  C. Test Results
  D. Risks/Follow-ups
```

---

## AI-1 启动指令（过滤体系：Set/Multi/Floating）

```text
你是 AI-1，负责过滤体系企业化。请立即开始，不要写方案，直接实施。

主计划：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

你的白名单文件：
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/AdvancedFilterPanel.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/FilterBar.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/SetFilterPanel.tsx (create)
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/MultiFilterPanel.tsx (create)
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/FloatingFilterRow.tsx (create)
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/advancedFilter.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-enterprise-filter-suite.spec.ts (create)

禁止修改：
- F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx
- 任何非白名单文件

先做这些步骤：
1) 在 advancedFilter 相关单测中补充失败用例：Set/Multi/Floating 与 Advanced 组合语义。
2) 跑测试确认失败。
3) 实现最小可行改动，直到单测通过。
4) 增加/完善 e2e/erp-data-grid-enterprise-filter-suite.spec.ts。
5) 跑全部指定测试并提交。

必须执行命令：
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/advancedFilter.test.ts src/components/erp-data-grid/hooks/useGridAdvancedFilter.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-enterprise-filter-suite.spec.ts --project=chromium

建议提交信息：
feat(grid-filter): add set/multi/floating filter enterprise behavior
```

---

## AI-2 启动指令（SSRM + 性能 + 契约韧性）

```text
你是 AI-2，负责 SSRM 与性能韧性。请立即开始，不要先写方案。

主计划：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

你的白名单文件：
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridServerSide.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/serverSide.test.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-server-side-resilience.spec.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-performance-tier.spec.ts
- F:/erp/apps/erp-web/docs/ErpDataGrid/PerformanceBaseline.md

禁止修改：
- F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx
- 任何非白名单文件

先做这些步骤：
1) 在 serverSide.test.ts 追加失败用例：参数契约、取消、重试、stale 响应隔离。
2) 跑测试确认失败。
3) 实现最小改动修复。
4) 更新 resilience/performance E2E 与性能基线文档。
5) 跑指定测试并提交。

必须执行命令：
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/serverSide.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-server-side-resilience.spec.ts --project=chromium
pnpm --filter erp-web run test:e2e:grid-performance-tier

建议提交信息：
feat(grid-ssrm): harden request lifecycle and tier performance stability
```

---

## AI-3 启动指令（分组/透视/树形/明细）

```text
你是 AI-3，负责分组透视与树明细一致性。直接实施，不写泛泛计划。

主计划：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

你的白名单文件：
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridGrouping.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridPivot.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/PivotPanel.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/pivot.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/aggregation.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridTree.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridDetail.test.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-group-pivot-enterprise.spec.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-tree-detail-enterprise.spec.ts

禁止修改：
- F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx
- 任何非白名单文件

先做这些步骤：
1) 补失败测试：subtotal/grandtotal 数学一致、树/明细状态恢复一致。
2) 跑测试确认失败。
3) 最小实现修复。
4) 跑透视与树明细 E2E 回归。
5) 提交。

必须执行命令：
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/pivot.test.ts src/components/erp-data-grid/utils/aggregation.test.ts src/components/erp-data-grid/hooks/useGridTree.test.ts src/components/erp-data-grid/hooks/useGridDetail.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-group-pivot-enterprise.spec.ts e2e/erp-data-grid-tree-detail-enterprise.spec.ts --project=chromium

建议提交信息：
feat(grid-pivot-tree): improve group/pivot/tree/detail consistency
```

---

## AI-4 启动指令（编辑/范围/剪贴板：Fill + Paste）

```text
你是 AI-4，负责编辑深水区：Fill Handle、Clipboard Paste、Undo/Redo 稳定性。直接开始实施。

主计划：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

你的白名单文件：
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/clipboard.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/ErpDataGridImpl.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridEditing.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridKeyboard.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/clipboard.test.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-range-editing.spec.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-fill-paste-enterprise.spec.ts (create)

禁止修改：
- F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx
- 任何非白名单文件

先做这些步骤：
1) 补失败测试：Paste、Fill Handle、Undo/Redo 历史一致性。
2) 跑测试确认失败。
3) 实现 paste 解析与范围写入，再实现 fill handle。
4) 修正键盘事件与 range 交互一致性。
5) 跑单测和 E2E，提交。

必须执行命令：
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/hooks/useGridEditing.test.ts src/components/erp-data-grid/hooks/useGridKeyboard.test.ts src/components/erp-data-grid/utils/clipboard.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-range-editing.spec.ts e2e/erp-data-grid-fill-paste-enterprise.spec.ts --project=chromium

建议提交信息：
feat(grid-editing): implement fill-handle and clipboard paste with stable undo/redo
```

---

## AI-5 启动指令（导出/图表/状态栏 + Sparkline）

```text
你是 AI-5，负责导出深度、图表深度、状态栏精度和 sparkline。直接实施。

主计划：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

你的白名单文件：
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/export.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/ChartPanel.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/StatusBar.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/chart.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/utils/summary.test.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-export-enterprise.spec.ts
- F:/erp/apps/erp-web/e2e/erp-data-grid-chart-status.spec.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/SparklineCell.tsx (create)
- F:/erp/apps/erp-web/e2e/erp-data-grid-sparkline.spec.ts (create)

禁止修改：
- F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx
- 任何非白名单文件

先做这些步骤：
1) 补失败测试：Excel 深度导出（样式/多 sheet/公式）与图表一致性。
2) 跑测试确认失败。
3) 最小实现导出深度能力和审计闭环。
4) 增加 sparkline 组件与 E2E。
5) 跑单测 + E2E 回归并提交。

必须执行命令：
pnpm --filter erp-web exec vitest --run src/components/erp-data-grid/utils/export.test.ts src/components/erp-data-grid/utils/chart.test.ts src/components/erp-data-grid/utils/summary.test.ts
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-export-enterprise.spec.ts e2e/erp-data-grid-chart-status.spec.ts e2e/erp-data-grid-sparkline.spec.ts --project=chromium

建议提交信息：
feat(grid-export-chart): deepen enterprise export/chart and add sparkline support
```

---

## AI-6 启动指令（总装集成 + 门禁）

```text
你是 AI-6，总装负责人。你是唯一可以改 ErpDataGrid.tsx 的 AI。直接实施，不先写泛化计划。

主计划：
F:/erp/docs/superpowers/plans/2026-04-10-erpdatagrid-enterprise-95-six-ai-industrial.md

你的白名单文件：
- F:/erp/apps/erp-web/src/components/ErpDataGrid.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/SidePanel.tsx
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/hooks/useGridSidePanel.test.ts
- F:/erp/apps/erp-web/src/components/erp-data-grid/components/ColumnSettingsPanel.tsx
- F:/erp/apps/erp-web/e2e/erp-data-grid-workbench-integration.spec.ts
- F:/erp/apps/erp-web/package.json
- F:/erp/docs/web/ERPDATAGRID_ANALYSIS_RELEASE_GATE.md
- F:/erp/apps/erp-web/e2e/erp-data-grid-enterprise-gate.spec.ts (create)
- F:/erp/docs/web/ERPDATAGRID_ENTERPRISE_95_RELEASE_CHECKLIST.md (create)

先做这些步骤：
1) 在等待其他 AI 期间先完成：门禁脚本纳管位、发布清单模板、side panel 行为测试增强。
2) 按合并顺序逐步集成 AI-1~AI-5 成果，遇冲突只在你轨道收口。
3) 将企业专项 E2E 纳入 gate 脚本（新增 test:gate:grid-enterprise-95 或纳入现有 gate）。
4) 执行全量验证并输出证据。
5) 提交集成分支。

必须执行命令：
pnpm --filter erp-web run typecheck
pnpm --filter erp-web run test:unit:grid-analysis
pnpm --filter erp-web run test:e2e:grid-analysis
pnpm --filter erp-web run test:e2e:grid-performance-tier
pnpm --filter erp-web exec playwright test e2e/erp-data-grid-enterprise-filter-suite.spec.ts e2e/erp-data-grid-group-pivot-enterprise.spec.ts e2e/erp-data-grid-export-enterprise.spec.ts e2e/erp-data-grid-chart-status.spec.ts e2e/erp-data-grid-range-editing.spec.ts e2e/erp-data-grid-fill-paste-enterprise.spec.ts e2e/erp-data-grid-tree-detail-enterprise.spec.ts e2e/erp-data-grid-sparkline.spec.ts e2e/erp-data-grid-workbench-integration.spec.ts e2e/erp-data-grid-enterprise-gate.spec.ts --project=chromium
pnpm --filter erp-web run test:gate:grid-analysis

建议提交信息：
feat(grid-integration): integrate 95% enterprise tracks and enforce industrial release gates
```

---

## 分发顺序建议

1. 先发“通用广播模板”给全部 AI。
2. 再分别发 AI-1 到 AI-6 专属模板。
3. 要求每个 AI 在首条回复先回显：
   - 目标
   - 白名单文件
   - 立即执行的第一条命令

