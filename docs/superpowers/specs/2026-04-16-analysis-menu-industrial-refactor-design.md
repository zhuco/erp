# 分析菜单工业级重构设计

**日期**: 2026-04-16
**适用范围**: `F:/erp/apps/erp-web`
**目标模块**: 顶部导航中的分析域，重构后建议改名为“洞察”

## 1. 背景

当前仓库中的分析域已经具备一批基础页面与接口：

- 统计页：`/stats/sales`、`/stats/sales-daily`、`/stats/sales-trends`、`/stats/shipping`、`/stats/slow-moving`、`/stats/best-sellers`、`/stats/sales-cycle`
- 报表页：`/reports`
- 导出页：`/exports`

但当前分析菜单仍然停留在“少量统计页 + 报表/导出入口”的初级阶段，无法承接工业级 ERP 中按经营问题组织的洞察工作流。

用户提供的参考截图体现了传统 ERP 中“报表很多、入口很密、粒度混杂”的分析菜单特征。该截图对本项目的参考价值，不在于复刻名称和排版，而在于反推出这些系统真正关心的分析问题：

1. 交易是否健康。
2. 履约是否及时。
3. 商品结构是否合理。
4. 库存是否安全。
5. 售后是否受控。
6. 标准报表与导出链路是否闭环。

本次重构不允许照搬截图中的名称，也不允许照搬其“多列铺满的报表清单”排版。我们要把这些传统报表能力收敛成一套更适合当前仓库、也更适合 Codex 并行实现的工业级洞察域。

## 2. 现状约束

### 2.1 当前前端架构

分析菜单由以下文件控制：

- 菜单树：`F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- 头部模块模型：`F:/erp/apps/erp-web/src/app/layout/headerMenuModel.ts`
- 共享二级面板逻辑：`F:/erp/apps/erp-web/src/app/layout/headerSharedSubmenu.ts`
- 路由入口：`F:/erp/apps/erp-web/src/app/router.tsx`
- 路由别名：`F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts`
- 页面标题：`F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`

### 2.2 当前分析域能力

分析相关前端实现当前主要分散在：

- 统计 API：`F:/erp/apps/erp-web/src/api/stats.ts`
- 报表 API：`F:/erp/apps/erp-web/src/api/reports.ts`
- 统计占位页骨架：`F:/erp/apps/erp-web/src/pages/stats/StatsPlaceholder.tsx`
- 已有统计页：
  - `F:/erp/apps/erp-web/src/pages/stats/SalesStatsPage.tsx`
  - `F:/erp/apps/erp-web/src/pages/stats/SalesDailyReportPage.tsx`
  - `F:/erp/apps/erp-web/src/pages/stats/SalesTrendAnalysisPage.tsx`
  - `F:/erp/apps/erp-web/src/pages/stats/ShippingStatsPage.tsx`
  - `F:/erp/apps/erp-web/src/pages/stats/SlowMovingAnalysisPage.tsx`
  - `F:/erp/apps/erp-web/src/pages/stats/BestSellerAnalysisPage.tsx`
  - `F:/erp/apps/erp-web/src/pages/stats/SalesCyclePage.tsx`
- 综合报表页：`F:/erp/apps/erp-web/src/pages/reports/ReportsPage.tsx`
- 导出页：`F:/erp/apps/erp-web/src/pages/exports/ExportJobsPage.tsx`

### 2.3 当前数据合同

当前统计合同支持的 `stats_type` 只有：

- `sales`
- `shipping`
- `sales-daily`
- `sales-trends`
- `slow-moving`
- `best-sellers`
- `sales-cycle`

当前 `StatsPlaceholder` 仅内建以下通用筛选能力：

- 时间范围
- 单一统计维度 `dimension`
- 指标选择 `metrics`
- 对比周期

这意味着本次重构第一阶段必须优先复用已有合同，不应一开始就引入过大的分析 API 重写。

## 3. 方案比选

### 方案 A：截图平移型

做法：

- 直接把截图中的报表名称搬运为新的菜单项。
- 在现有“分析”模块下继续堆叠大量二级入口。
- 先不改路由语义，只补若干占位页。

优点：

- 上手快。
- 菜单数量增长直接。

缺点：

- 名称沿袭旧 ERP 报表思路，不符合本项目当前 IA。
- 同一业务问题会被拆成“明细账/汇总账/货品/单据”等多个入口，用户理解成本高。
- 不利于 Codex 多子任务并行拆分，容易形成大量薄页面和重复逻辑。

结论：

- 不推荐。

### 方案 B：路由换新型

做法：

- 不复刻截图名称，但仍以“报表清单”为核心组织方式。
- 统一迁移为新路由前缀，例如 `/insights/*`。
- 页面按老报表颗粒度继续存在。

优点：

- 兼顾了一部分重命名和 canonical route 治理。
- 迁移成本中等。

缺点：

- 只换了壳，没有真正按经营问题重构信息架构。
- 仍然会留下过多“账表碎片入口”。

结论：

- 可作为过渡，但不够工业级。

### 方案 C：经营问题导向型

做法：

- 将顶级模块从“分析”升级为“洞察”。
- 不再按截图中的“仓储/销售/货品/库存/服务”原始分区组织，而改为按经营问题组织：
  - 收入与订单质量
  - 履约与仓配效率
  - 商品结构与生命周期
  - 库存健康与调拨
  - 售后与补偿治理
  - 报表与导出工坊
- 将截图中大量“汇总/明细/货品/单据”的老报表合并为更清晰的统一能力面。
- 第一期复用现有 `/stats`、`/reports`、`/exports`；第二期再扩展新合同。

优点：

- 和当前仓库的菜单、路由、测试体系更兼容。
- 对用户更稳定，入口围绕问题而非围绕报表口径碎片。
- 更适合 Codex 并行拆任务：导航、合同、领域页、测试可以分开。

缺点：

- 需要做一轮能力映射与命名收敛。
- 需要对 legacy 页面建立新旧路由别名关系。

结论：

- **推荐采用本方案。**

## 4. 新的信息架构

## 4.1 模块命名

- 当前顶级菜单 `分析`
- 重构后建议改为 `洞察`

原因：

- “洞察”强调经营判断与决策支持，不是简单“报表堆叠”。
- 和截图中的传统“统计菜单”形成明显区分。

## 4.2 菜单布局

本次不沿用截图那种“大面积平铺报表清单”的排版，改为两层结构：

### 头部共享面板

仅显示稳定的业务组入口：

1. 总览快报
2. 收入与订单质量
3. 履约与仓配效率
4. 商品结构与生命周期
5. 库存健康与调拨
6. 售后与补偿治理
7. 报表与导出工坊

### 洞察落地页 `/insights`

展示：

1. 本周关注指标条
2. 领域卡片入口
3. 最近使用
4. 异常提示
5. 导出与订阅入口

这样既保留头部快速跳转，又避免把所有功能都塞进共享二级菜单里。

## 4.3 路由策略

建议引入新的 canonical route：

- `/insights`
- `/insights/revenue/*`
- `/insights/fulfillment/*`
- `/insights/catalog/*`
- `/insights/inventory/*`
- `/insights/service/*`
- `/insights/workbench/*`

兼容策略：

- 现有 `/stats/*` 保留访问能力，但重定向到对应 `/insights/*`
- 现有 `/reports` 重定向到 `/insights/workbench/report-center`
- 现有 `/exports` 重定向到 `/insights/workbench/export-queue`

## 5. 第一阶段能力映射原则

### 5.1 同义报表合并

截图中大量条目只是在以下维度上拆分：

- 汇总 vs 明细
- 单据 vs 货品
- 日报 vs 统计
- 某业务对象的不同切片

本方案不按旧报表颗粒度一一复制，而是将其合并为“一个业务问题 + 多种视图”的统一能力面。

例如：

- `发货笔汇总账`、`发货货品汇总账`、`发货单明细账`、`发货单明细账_货品`
  - 合并为：`收发作业流水`
- `盘点汇总账_货品`、`盘点汇总账_单据`、`盘点明细账_货品`、`盘点明细账_单据`
  - 合并为：`盘点偏差`
- `售后退货明细账`、`售后退货汇总账`
  - 合并为：`退回闭环`

### 5.2 只保留可推断功能

凡是无法稳定推断统计口径、业务主语或指标定义的功能，不纳入当前文档。

本轮明确不纳入第一阶段设计正文的条目：

- `上报销售单汇总`
  - “上报”业务语义不清，无法确定是渠道回传、上级汇报还是财务报送。
- `货品供应商`
  - 更像主数据查询，不像统计页面。
- `货品存货仓库`
  - 更像主数据映射，不像洞察页面。

## 6. 可推断功能与统计维度

下表列出本轮可稳定推断的能力面。命名均为新的 canonical 名称，不复用截图中的原始命名。

| 领域组 | 新能力名 | canonical route | 主要解决的问题 | 统计维度 | 核心指标 | 主数据依赖 | 置信度 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 总览快报 | 交易总览 | `/insights/revenue/overview` | 当前交易规模和订单质量是否健康 | 日期、平台、店铺、订单来源、订单状态 | GMV、订单量、客单价、退款率 | 平台、店铺、订单主表 | 高 |
| 总览快报 | 营运日报 | `/insights/revenue/daily-brief` | 每日经营是否偏离目标 | 日期、店铺、仓库、班组 | 日 GMV、日订单数、日发货及时率、日退款率、异常单量 | 店铺、仓库、订单/发货汇总 | 中 |
| 收入与订单质量 | 增长雷达 | `/insights/revenue/trend-radar` | 销售增长是否持续 | 日期粒度、平台、店铺、品类 | 同比、环比、7 日移动均值、峰值 GMV | 日期维表、平台、品类 | 高 |
| 收入与订单质量 | 让利诊断 | `/insights/revenue/discount-insight` | 折扣是否侵蚀收入 | 平台、店铺、活动、SKU、品类 | 折扣金额、折扣率、折扣订单量、净销售额 | 活动、SKU、订单明细 | 高 |
| 收入与订单质量 | 订单结构 | `/insights/revenue/order-mix` | 订单与售出结构是否合理 | 平台、店铺、品类、品牌、SKU、订单来源 | 订单量、售出件数、GMV、结构占比 | SKU、品类、品牌、订单明细 | 高 |
| 履约与仓配效率 | 履约时效 | `/insights/fulfillment/lead-time` | 发货是否及时，承诺是否达成 | 仓库、承运商、平台、店铺、日期、承诺类型 | 发货单量、平均出库时长、准时发货率、承诺达成率 | 仓库、承运商、发货单、物流轨迹 | 高 |
| 履约与仓配效率 | 订单履约周期 | `/insights/fulfillment/order-cycle` | 订单全链路履约周期是否可控 | 平台、店铺、仓库、物流产品、日期 | 下单到发货时长、发货到签收时长、退货处理周期、按时履约率 | 订单、发货、签收、售后完结记录 | 高 |
| 履约与仓配效率 | 收发作业流水 | `/insights/fulfillment/operation-ledger` | 收货/发货作业量和对象分布是什么 | 单据类型、业务单据、仓库、日期、SKU、操作员 | 单据数、件数、收货量、发货量 | 仓库、SKU、出入库单据、操作员 | 高 |
| 履约与仓配效率 | 包装耗用 | `/insights/fulfillment/packaging-consumption` | 包材是否消耗异常 | 仓库、包材类型、日期、操作员、发货渠道 | 包材用量、包材成本、单均耗材、发货单量 | 包材主数据、仓库、发货单 | 高 |
| 履约与仓配效率 | 盘点偏差 | `/insights/fulfillment/stocktake-variance` | 盘点差异发生在哪些对象上 | 仓库、盘点批次、SKU、盘点单据、日期、操作员 | 账面量、实盘量、差异量、差异金额、差异率 | 仓库、盘点单、SKU | 高 |
| 履约与仓配效率 | 作业准确度 | `/insights/fulfillment/accuracy-control` | 拣配与发运错漏是否可控 | 仓库、配货员、打包员、班次、错误类型、日期 | 错漏数、错发率、漏发率、覆盖率 | 操作员、发货单、作业日志 | 中高 |
| 履约与仓配效率 | 仓网效能 | `/insights/fulfillment/warehouse-productivity` | 仓库团队效率是否达标 | 仓库、团队、班次、日期 | 处理单量、处理行数、人均产能、时效达成率 | 仓库、班组、作业统计 | 中 |
| 商品结构与生命周期 | 爆品热度 | `/insights/catalog/hot-sku` | 哪些商品在驱动收入 | SKU、SPU、品类、品牌、店铺、平台、日期 | 销售额、订单量、贡献率、复购率 | SKU、品类、品牌、销售明细 | 高 |
| 商品结构与生命周期 | 长尾沉积 | `/insights/catalog/slow-sku` | 哪些商品形成滞销与库存占压 | SKU、品类、品牌、仓库、生命周期阶段 | 滞销库存额、滞销 SKU 数、周转天数、呆滞占比 | SKU、库存余额、生命周期规则 | 高 |
| 商品结构与生命周期 | 连带组合 | `/insights/catalog/basket-links` | 哪些商品适合联动销售 | 主 SKU、连带 SKU、品类、店铺、平台、日期 | 共购次数、连带率、组合销售额 | 订单行、SKU 关系 | 高 |
| 库存健康与调拨 | 收发存总览 | `/insights/inventory/stock-balance` | 库存余额如何随收发变化 | 仓库、SKU、品类、日期、事务类型 | 期初量、入库量、出库量、期末量、净变化量 | 仓库、SKU、库存流水 | 高 |
| 库存健康与调拨 | 库龄健康 | `/insights/inventory/age-health` | 老库存和呆滞风险在哪里 | 仓库、SKU、品类、库龄区间、批次 | 在库量、库龄库存额、呆滞占比、平均库龄天数 | 批次、库存余额、库龄规则 | 高 |
| 库存健康与调拨 | 周转流速 | `/insights/inventory/flow-velocity` | 库存流动速度是否异常 | 仓库、SKU、品类、库存状态 | 周转天数、流速、现货、可用量、锁定量 | 库存余额、出库明细 | 高 |
| 库存健康与调拨 | 调拨效益 | `/insights/inventory/transfer-economics` | 调拨是否提升履约与库存平衡 | 调出仓、调入仓、SKU、调拨类型、在途状态、日期 | 调拨量、在途量、在途天数、调拨成本、调拨收益差 | 仓库、调拨单、在途记录 | 中高 |
| 库存健康与调拨 | 负库存预警 | `/insights/inventory/negative-balance-alert` | 负库存是否被及时发现和处理 | 仓库、SKU、事务来源、日期 | 负结余次数、负结余量、未恢复时长 | 库存流水、异常规则 | 高 |
| 售后与补偿治理 | 退回闭环 | `/insights/service/returns` | 退货处理是否闭环 | 原因、仓库、店铺、平台、SKU、日期、质检员 | 退货单量、退货件数、质检合格率、平均退回周期 | 售后单、退货入库、质检单 | 高 |
| 售后与补偿治理 | 补偿看板 | `/insights/service/compensation` | 补偿退款是否失控 | 补偿原因、店铺、平台、审核人、日期 | 补偿单量、补偿金额、单均补偿、审核周期 | 售后单、退款单、审核记录 | 高 |
| 售后与补偿治理 | 补发质量 | `/insights/service/reship-quality` | 补发和错漏发是否持续发生 | 原因、仓库、操作员、店铺、平台、SKU、日期 | 补发单量、错发数、漏发数、平均补发周期 | 售后单、发货单、异常记录 | 高 |
| 售后与补偿治理 | 工单效率 | `/insights/service/ticket-efficiency` | 客服工单处理是否达标 | 工单类型、服务团队、坐席、优先级、渠道、日期 | 工单量、首次响应时长、结案时长、SLA 达成率 | 服务工单、客服组织、SLA 规则 | 高 |
| 报表与导出工坊 | 报表中心 | `/insights/workbench/report-center` | 汇总经营、履约、库存和财务标准报表 | 时间范围、平台、店铺、库存预警阈值 | 店铺经营汇总、履约超时量、库存预警、财务差异 | 订单、库存、财务、履约汇总 | 高 |
| 报表与导出工坊 | 导出队列 | `/insights/workbench/export-queue` | 导出任务是否可追踪 | 任务类型、平台、店铺、仓库、日期范围、状态 | 任务数、完成率、失败率、平均时长 | 导出任务、对象存储、审计日志 | 高 |

功能细化规格见：

- `F:/erp/docs/spec/18_insights_function_details.md`

## 7. 统一维度字典

为了让不同洞察页面复用筛选器和接口语义，建议建立统一维度字典 `insightDimensionCatalog`。

### 7.1 时间维

- `day`
- `week`
- `month`
- `custom_range`

### 7.2 交易维

- `platform`
- `shop`
- `order_source`
- `order_status`
- `campaign`
- `promise_type`

### 7.3 履约维

- `warehouse`
- `courier`
- `operator`
- `shift`
- `document_type`
- `stocktake_batch`

### 7.4 商品维

- `sku`
- `spu`
- `category`
- `brand`
- `lifecycle_stage`
- `supplier`

### 7.5 库存维

- `transaction_type`
- `stock_state`
- `age_bucket`
- `lot`
- `source_warehouse`
- `target_warehouse`

### 7.6 售后维

- `reason_code`
- `ticket_type`
- `service_team`
- `agent`
- `inspector`
- `approver`

## 8. 数据接入策略

第一阶段不引入“万能分析接口”，而是沿用当前已存在的三类合同：

### 8.1 KPI/趋势类

沿用：

- `GET /v1/tenants/{tenant_id}/stats/{stats_type}`

适用：

- 交易总览
- 营运日报
- 增长雷达
- 履约时效
- 爆品热度
- 长尾沉积
- 履约周期类页面

### 8.2 标准报表类

沿用：

- `reports` 系列接口

适用：

- 收发存总览
- 库龄健康
- 报表中心

### 8.3 导出任务类

沿用：

- `exports` 系列接口

适用：

- 导出队列

### 8.4 第二阶段扩展

以下能力大概率需要新增合同或扩展响应结构：

- 让利诊断
- 收发作业流水
- 包装耗用
- 盘点偏差
- 作业准确度
- 仓网效能
- 连带组合
- 调拨效益
- 退回闭环
- 补偿看板
- 补发质量
- 工单效率

原因不是无法命名，而是这些页面更接近“专业分析页”或“主题台账页”，需要更丰富的维度明细与分页能力。

## 9. 分阶段落地建议

### 9.1 Phase 1：导航与现有页面工业化收口

目标：

1. 将 `分析` 模块升级为 `洞察`
2. 引入 `/insights/*` canonical route
3. 将现有 `/stats/*`、`/reports`、`/exports` 接到新 IA
4. 建立能力目录、维度字典、路由别名和标题映射

优先能力：

- 交易总览
- 营运日报
- 增长雷达
- 履约时效
- 爆品热度
- 长尾沉积
- 履约周期
- 报表中心
- 导出队列

### 9.2 Phase 1.5：复用现有数据源新增主题页

目标：

- 在不重做分析引擎的前提下，补齐结构分布、收发存总览、库龄健康、周转流速等页面。

### 9.3 Phase 2：主题台账与专业分析扩展

目标：

- 新增履约台账、盘点偏差、调拨效益、退回闭环、补发质量等专业分析页。

## 10. 共享文件锁

以下文件属于本次重构的高冲突共享文件，默认由主控会话串行修改，不直接分配给并行 Worker：

- `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- `F:/erp/apps/erp-web/src/app/layout/headerMenuModel.ts`
- `F:/erp/apps/erp-web/src/app/router.tsx`
- `F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts`
- `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
- `F:/erp/apps/erp-web/src/api/stats.ts`
- `F:/erp/apps/erp-web/src/api/reports.ts`

这些文件需要采用“主控会话收口 + Worker 只改自己负责领域页”的方式，避免多个 Codex 子任务互相踩路由、菜单和合同。

## 11. 测试与回归要求

### 11.1 菜单与导航

- 单测：
  - `F:/erp/apps/erp-web/src/app/layout/headerMenuModel.test.ts`
  - `F:/erp/apps/erp-web/src/app/layout/headerSharedSubmenu.test.ts`
- E2E：
  - `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`

### 11.2 分析页

- E2E：
  - `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`

### 11.3 兼容性

新旧地址兼容必须至少覆盖：

- `/stats/*` -> `/insights/*`
- `/reports` -> `/insights/workbench/report-center`
- `/exports` -> `/insights/workbench/export-queue`

## 12. 设计结论

本次分析菜单重构的本质，不是把截图中的旧报表重新摆一遍，而是：

1. 把传统报表能力重新组织成“经营问题导向”的洞察域。
2. 用新的 canonical route 与 feature catalog 承接现有实现。
3. 用统一维度字典约束未来统计页。
4. 用共享文件锁和任务分包支持 Codex 多子任务并行开发。

下一步应基于本设计，生成一份明确的多子任务实施计划，指导菜单、路由、领域页、合同和测试分别由不同 worker 并行推进。
