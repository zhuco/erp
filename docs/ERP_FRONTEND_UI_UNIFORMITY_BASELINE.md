# ERP 前端 UI 统一性基线报告

> 本报告由 `apps/erp-web/scripts/assess-ui-uniformity.mjs --json` 生成数据后整理，目标是为后续“95% 以上用户感知统一性”提供可复现基线。  
> 最近评估时间：`2026-05-17`。当前数据反映本轮批量迁移后的工作区状态。

## 1. 最新结论

- 页面总数：`215`；治理适用页面：`180`；baseline 例外：`30`。
- 模板治理口径采用率：`180/180`，即 `100.00%`，高于当前硬门槛 `95.00%`。
- UI 统一性综合评分：`68.5 / 100`，目标 `95`，差距 `26.5`，当前状态：`未达标`。
- 结论：项目已经完成“模板接入治理达标”，但距离“ERP 用户感知统一性 95%+”仍有明显差距。当前主要差距集中在操作流程统一、表格体验统一、视觉/文案统一和可访问性/键盘效率四个维度。
- 下一阶段不要再把“是否接入模板”作为主要矛盾，应继续压降裸 `Table/Card`、提高 P0 主表工业表格契约覆盖、补统一工具栏与统一状态区，并把评分脚本逐步接入门禁。

## 2. 当前核心指标

| 指标 | 当前值 | 比例/说明 |
| --- | ---: | --- |
| 页面总数 | 215 | 扫描 `src/pages/**/*Page.tsx` |
| 适用页面数 | 180 | 列表/工作台/设置等纳入治理页面 |
| baseline 例外 | 30 | 13.95% |
| 治理口径模板采用 | 180/180 | 100.00% |
| 直接推荐模板采用 | 175 | 97.22% |
| WorkbenchPageTemplate 直接采用 | 31 | 14.42% |
| UiModule03MainTableSection | 106 | 49.30% |
| 裸 Table 页面 | 50 | 25.58% |
| 裸 Card 页面 | 88 | 51.16% |
| industrial-review-compact | 80 | 37.21% |
| selectionCellMode="custom" | 80 | 统一选择列契约 |
| selectionHeaderMode="allInvert" | 80 | 全选/反选表头契约 |
| WorkbenchToolbar / toolbar slots | 104 | 统一工具栏显式标识 |
| data-testid | 160 | 稳定测试选择器覆盖 |
| 列设置相关标识 | 115 | 列设置/视图配置治理线索 |
| 左侧面板 | 30 | 左侧筛选/导航模块 |
| 右侧面板 | 33 | 详情/辅助信息模块 |
| 底部面板 | 36 | 明细/日志/子表模块 |

## 3. 评分拆分

| 维度 | 权重 | 当前得分 | 与满分差距 | 说明 |
| --- | ---: | ---: | ---: | --- |
| 模板接入统一性 | 20 | 20.0 | 0.0 | 治理口径模板采用率已保持 100% |
| 操作流程统一性 | 25 | 10.9 | 14.1 | 工具栏、主操作、筛选区、稳定测试入口仍需继续统一 |
| 表格体验统一性 | 20 | 10.6 | 9.4 | 工业紧凑表格、统一选择、全选/反选和裸 Table 收敛仍未完成 |
| 视觉与文案统一性 | 15 | 11.0 | 4.0 | 裸 Card、局部自组装 section、状态文案和页面说明仍需收敛 |
| 可访问性与键盘效率 | 10 | 4.9 | 5.1 | 焦点返回、键盘关闭、可访问名称和跨页快捷键仍缺门禁 |
| 工程治理与回归门禁 | 10 | 9.7 | 0.3 | 模板采用、UTF-8、lint/typecheck/build 基础良好，评分门禁仍需固化 |
| **合计** | **100** | **68.1** | **26.9** | 当前仍需约 26.9 分提升才能达到 95 目标 |

评分说明：深化评分包含模板接入 20 分、操作流程 25 分、表格体验 20 分、视觉文案 15 分、可访问性与键盘 10 分、工程治理 10 分；当前分数用于基线和趋势，不直接替代人工验收。

## 4. P0 摘要

| 指标 | 当前值 | 说明 |
| --- | ---: | --- |
| P0 总页 | 82 | 高频 ERP 操作页面 |
| P0 适用页 | 80 | 纳入统一治理页面 |
| P0 模板治理页 | 80 | 100.00% |
| P0 WorkbenchPageTemplate | 13 | 15.85% |
| P0 UiModule03MainTableSection | 67 | 81.71% |
| P0 裸 Table 页 | 6 | 13.41%，下一阶段优先清零 |
| P0 裸 Card 页 | 21 | 52.44%，下一阶段优先降到 30 以下 |
| P0 industrial-review-compact | 71 | 86.59% |
| P0 WorkbenchToolbar / toolbar slots | 79 | 74.39% |
| P0 data-testid | 79 | 96.34% |
| P0 列设置相关标识 | 73 | 89.02% |
| P0 baseline 例外 | 2 | 不得新增 |

## 5. 当前差距判断

### 5.1 距离 95 分还差 26.9 分

当前模板接入已经满分，后续提升不能依赖“再套模板”获得主要收益。要达到 95，需要重点补齐以下短板：

1. **操作流程统一性差 14.1 分**：仍有大量页面未显式统一 `toolbarLeft/toolbarRight/actionBar/header`，用户在不同模块中寻找查询、刷新、新建、导出、列设置的位置仍有成本。
2. **表格体验统一性差 9.4 分**：P0 仍有 `6` 个裸 Table 页面，全量仍有 `50` 个裸 Table 页面；明细表、抽屉表、弹窗表需要统一为工业表格或底部表格契约。
3. **视觉与文案统一性差 4.3 分**：全量仍有 `110` 个裸 Card 页面，P0 仍有 `43` 个裸 Card 页面；很多页面仍用局部 Card 自组装标题、筛选、局部表格和状态区域。
4. **可访问性与键盘效率差 5.1 分**：当前评分主要通过代理指标估算，仍缺明确的焦点、键盘、`aria-label`、弹层关闭回焦门禁。

### 5.2 已经明显改善的部分

- 模板治理口径已经达到 `100.00%`，不再是主要风险。
- P0 `data-testid` 覆盖已到 `96.34%`，基本达到可测性目标。
- P0 工业表格契约覆盖达到 `86.59%`，下一步适合以剩余 P0 裸 Table 页面为突破口。
- 本轮多页面改造后，P0 裸 Table 已降到 `11`，具备下一轮清零条件。

## 6. 下一批 P0 页面清单

| 优先级 | 页面 | 当前主要风险 |
| ---: | --- | --- |
| 1 | `src/pages/products/ProductBarcodesPage.tsx` | 裸 Table、裸 Card、仍需直接工作台化 |
| 2 | `src/pages/products/ProductChannelAvailabilityPage.tsx` | 裸 Table、裸 Card、仍需直接工作台化 |
| 3 | `src/pages/wms/StockCountsPage.tsx` | 裸 Table、裸 Card、仍需直接工作台化 |
| 4 | `src/pages/wms/StockMovesPage.tsx` | 裸 Table、裸 Card、仍需直接工作台化 |
| 5 | `src/pages/wms/StockTransfersPage.tsx` | 裸 Table、裸 Card、仍需直接工作台化 |
| 6 | `src/pages/finance/TaxManagementPage.tsx` | 裸 Card、缺少统一工具栏标识 |
| 7 | `src/pages/inventory/InventoryCostsPage.tsx` | 裸 Card、缺少统一工具栏标识 |
| 8 | `src/pages/inventory/InventoryLedgerPage.tsx` | 裸 Card、缺少统一工具栏标识 |
| 9 | `src/pages/inventory/InventoryPage.tsx` | 裸 Card、缺少统一工具栏标识 |
| 10 | `src/pages/inventory/InventorySyncJobsPage.tsx` | 裸 Card、缺少统一工具栏标识 |
| 11 | `src/pages/orders/OrderReviewRulesPage.tsx` | 裸 Card、缺少统一工具栏标识 |
| 12 | `src/pages/procurement/ProcurementInspectionsPage.tsx` | 裸 Card、缺少统一工具栏标识 |

## 7. 下一步建议

1. **P0 裸 Table 清零批次**：优先处理 `ProductBarcodesPage`、`ProductChannelAvailabilityPage`、`StockCountsPage`、`StockMovesPage`、`StockTransfersPage`，每页补契约测试并禁止裸 `<Table`。
2. **P0 裸 Card 压降批次**：把只承载标题、筛选、工具栏、局部表格的裸 Card 迁入 `toolbarLeft`、`toolbarRight`、`header`、`actionBar`、`bottomPanel` 或页面内统一 section。
3. **工具栏补齐批次**：对 `TaxManagementPage`、`InventoryCostsPage`、`InventoryLedgerPage`、`InventoryPage`、`InventorySyncJobsPage` 等页面补 `toolbarLeft/toolbarRight` 和稳定 `data-testid`。
4. **契约测试固化**：每个迁移页面新增 `.contract.test.ts`，至少断言主表契约、工具栏插槽、列设置、禁止裸 Table/Card。
5. **门禁升级**：评分脚本先作为趋势门禁输出，再逐步提高阈值；建议先设置 P0 裸 Table 为阻断项，再设置 P0 裸 Card 阈值。

## 8. 复现命令

```bash
node apps/erp-web/scripts/assess-ui-uniformity.mjs
node apps/erp-web/scripts/assess-ui-uniformity.mjs --json
pnpm --dir apps/erp-web run lint:page-templates
pnpm --filter erp-web lint
pnpm --filter erp-web typecheck
pnpm --filter erp-web build
pnpm --dir F:\erp run check:utf8
```

JSON 结果可以被 `JSON.parse` 直接解析，适合后续接入 CI、趋势报告或 PR 注释。


## 9. 本轮开发后状态（2026-05-17）

- 本轮继续完成 `ProductBarcodesPage`、`ProductChannelAvailabilityPage`、`StockCountsPage`、`StockMovesPage`、`StockTransfersPage`、`TaxManagementPage`、`InventoryCostsPage`、`InventoryLedgerPage`、`InventoryPage`、`InventorySyncJobsPage` 共 `10` 个 P0 页面统一改造。
- 最新评分为 `68.5 / 100`，距离 `95` 仍差 `26.5`；本轮已推进但尚未达到 95，不能将模板治理 100% 等同于用户感知统一性达标。
- P0 裸 Card 从 `43` 降至 `33`，P0 裸 Table 从 `11` 降至 `6`，P0 统一工具栏标识从 `61` 提升至 `66`。
- 下一批优先处理评分脚本 `nextBatch.P0`：`OrderReviewRulesPage`、采购域 `Procurement*`、`PurchaseOrdersPage`、`PurchaseSuggestionsPage`、`SupplierPerformancePage`、商品域 `ProductChangeControlPage`、`ProductLifecyclePage`、`ProductUnitsPage`。


## 10. 本轮继续开发结果（2026-05-17）

- 本轮继续迁移 `OrderReviewRulesPage`、采购域 8 页、`SyncJobRunsPage`、`InvoicesPage`、`ProductPackagingSpecsPage` 以及商品治理 3 页的主表工具栏/筛选区/裸 Card 契约，共新增覆盖 `16` 个页面。
- 最新评分为 `68.5 / 100`，目标 `95`，差距 `26.5`；由于评分模型还强依赖全量裸 Table、P1 覆盖、键盘/a11y 与视觉治理，单轮大量 P0 工具栏迁移无法带来 10 分级跃升。
- P0 当前：裸 `Table` 为 `3`，裸 `Card` 为 `11`，统一工具栏标识为 `79/82`，P0 工具栏覆盖率已达到 `96.34%`。
- 下一步提分重点应从 P0 工具栏转向：清零 P0 裸 `Table`、压降商品治理页剩余详情裸 `Card`、批量推进 P1 裸 `Table/Card`、补键盘/a11y 门禁。


## 11. P0 裸表格继续压降（2026-05-17）

- 已将 `PurchaseReceiptsPage`、`PurchaseReturnsPage`、`LogisticsUploadPage` 中的页面级裸 `<Table>` 明细区迁移为统一 `ErpDataGrid`。
- 最新评分为 `68.5 / 100`，目标 `95`，差距 `26.5`，仍为 `未达标`。
- 全量裸 `Table` 从 `50` 降至 `41`，P0 裸 `Table` 从 `6` 降至 `3`；P0 仍需继续清理剩余裸表和裸 Card。
- 下一轮建议优先处理 `SalesProductMatchPage`、`ReconcilePage`、商品治理详情裸 Card、以及 WMS 策略页中的裸 Card/低一致性区域。


## 12. Reconcile / WMS / P1 样本继续治理（2026-05-17）

- 已解包 `ReconcilePage` 的 4 个表单裸 Card、商品治理页剩余主流程/详情裸 Card、WMS 策略/列表页筛选裸 Card。
- 已将 6 个规划 P1 样本页从裸 `Table/Card` 迁移为 `ErpDataGrid + section`，并补齐契约测试。
- 最新评分为 `68.5 / 100`，目标 `95`，差距 `26.5`，仍为 `未达标`。
- 全量裸 `Table` 降至 `41`，全量裸 `Card` 降至 `72`；P0 裸 `Card` 降至 `11`，P1 裸 `Table` 降至 `28`、P1 裸 `Card` 降至 `50`。
- 下一轮应继续处理 `MarketingPriceListsPage`、`SystemSyncJobsPage`、`PrintJobsPage`、`SkuListPage`、`ReportsPage`，并开始补键盘/a11y 门禁，否则评分中的 `a11yKeyboardConsistency=4.9/10` 会持续卡住。
