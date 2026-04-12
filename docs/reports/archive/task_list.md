# 缺口任务卡清单
> 状态声明（2026-04-10）：本文件为阶段性任务拆分记录，部分条目已在后续迭代完成。  
> 当前任务完成与发布口径请优先参考 `docs/ops/TASK_COMPLETION_MATRIX.md` 与 `docs/ops/RELEASE_GATE_CHECKLIST.md`。  
> 仅列出当前“部分/缺失”的关键闭环任务，按模块分组，便于多人并行。

## 并行拆分（4个AI工作包）
> 每个工作包要求完成后端/前端/脚本/迁移的闭环交付，下方任务卡已标注分工包。

### AI-1 基础账号与系统入口
- 覆盖任务: ACC-001, SYS-002, REP-001, OBS-001
- 交付细节: 账号资料/密码流程、权限种子与菜单入口、导出任务落地与存储、监控仪表盘与告警配置
- 协作接口: 对齐各业务模块菜单权限 code 清单
- 状态: 已完成（ACC-001/SYS-002/REP-001/OBS-001 已核验）

### AI-2 订单审核与仓储履约
- 覆盖任务: OMS-001, OMS-002, OMS-003, WMS-002, WMS-003, SHIP-001
- 交付细节: 审核规则与异常池、多包裹出库与包裹明细、退款回滚、波次生成、打包员记录、发货确认与追踪
- 协作接口: 与 AI-3 对齐物流规则与打印入口

### AI-3 物流/打印/售后
- 覆盖任务: LOG-001, LOG-002, PRINT-001, PRINT-002, AF-001
- 交付细节: 物流目录/资费、打印桥接与打印联动、退货入库回补库存
- 协作接口: 与 AI-2 对齐发货/波次流程

### AI-4 采购与财务
- 覆盖任务: PROC-001, FIN-001
- 交付细节: 应付闭环、支付对账
- 协作接口: 与 AI-1 对齐权限/菜单入口
- 状态: 已完成（PROC-001/FIN-001 已核验）

## 账号与权限
### ACC-001 登录账户管理/个人中心
- 目标: 提供个人信息、修改密码、退出登录等基础账户管理能力。
- 涉及: 表 `users`/`user_roles`；API `/me`、`/me/password`（或 `/users/{user_id}`）；UI `/account/profile` 或 `/system/profile`。
- 分工包: AI-1（基础账号与系统入口）
- 验收: 登录用户可更新个人信息并修改密码，重新登录成功。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/me`/`/me/password`；`apps/erp-web/src/pages/system/ProfilePage.tsx`；`apps/erp-web/src/api/auth.ts`；`apps/erp-web/src/app/router.tsx`
- 负责人: 已完成

### SYS-002 权限种子/菜单入口补齐
- 目标: 默认管理员可见订单审核、物流管理/资费、波次/发货/打包、打印、售后、采购财务、系统配置等入口。
- 涉及: `permissions`/`role_permissions`、`docs/dev/seed.sql`、`apps/erp-web` 菜单与 PermissionGuard。
- 分工包: AI-1（基础账号与系统入口）
- 验收: 使用 seed 账号登录即可访问上述页面。
- 状态: 已完成
- 证据: `docs/dev/seed.sql` 权限种子；`apps/erp-web/src/app/layout/menu.tsx`；`apps/erp-web/src/app/router.tsx`
- 负责人: 已完成

## 订单/审核
### OMS-001 订单审核/异常池管理台
- 目标: 管理审核规则并驱动异常池闭环。
- 涉及: 表 `order_review_rules`/`order_review_hits`/`exception_reasons`；API `/oms/review-rules` `/orders/exceptions`；UI `/orders/review`。
- 分工包: AI-2（订单审核与仓储履约）
- 验收: 规则命中订单进入异常池并可处理。
- 负责人: TBD

### OMS-002 多包裹出库与包裹明细
- 目标: 包裹维度出库与包裹明细维护闭环。
- 涉及: `pack_records`/`shipment_packages`；API `/shipments/{id}/packages`；UI 订单/发货页。
- 分工包: AI-2（订单审核与仓储履约）
- 验收: 包裹明细可维护且订单状态更新。
- 负责人: TBD

### OMS-003 未打印未发货订单自动退款回滚
- 目标: 退款/关闭订单时自动标记已退款并回滚仓储业务。
- 涉及: `orders`/`outbound_docs`/`pack_records`/`print_jobs`；事件 `ORDER_REFUNDED_AUTO`；Worker 回滚逻辑。
- 分工包: AI-2（订单审核与仓储履约）
- 验收: 未发货订单退款后仓储记录清理，订单状态更新。
- 负责人: TBD

## 仓储/WMS 与履约
### WMS-002 波次生成规则/自动化
- 目标: 按截止时间、单量上限、仓库策略自动生成波次。
- 涉及: `wave_rules`（或 settings）；API `/wms/waves/run`；Worker 定时任务；UI `/wms/waves`。
- 分工包: AI-2（订单审核与仓储履约）
- 验收: 波次可按规则生成。
- 负责人: TBD

### WMS-003 打包员登记/可追踪
- 目标: 打包记录关联操作人，支持追踪与统计。
- 涉及: `pack_records.operator_id`；API `/wms/outbounds/{id}/pack`；UI `WmsPackPage`。
- 分工包: AI-2（订单审核与仓储履约）
- 验收: 打包记录包含操作人且可查询。
- 负责人: TBD

### SHIP-001 发货确认/状态回写闭环
- 目标: 发货确认、状态回写、失败重试与追踪完整。
- 涉及: `shipment_push_attempts`；API `/shipments/{id}/confirm`；事件 `SHIPMENT_CONFIRMED`；UI `/wms/ship` `/shipments`。
- 分工包: AI-2（订单审核与仓储履约）
- 验收: 发货后状态可查且可重试回写。
- 负责人: TBD

## 物流管理
### LOG-001 物流目录管理
- 目标: 承运商/物流产品/匹配规则配置与权限入口完整。
- 涉及: 表 `carriers`/`logistics_products`/`logistics_match_rules`；API `/logistics/*`；UI `/logistics/products` `/logistics/matching-rules`。
- 分工包: AI-3（物流/打印/售后）
- 验收: 管理员可维护物流目录并应用匹配规则。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/logistics/products` `/logistics/matching-rules`；`apps/erp-web/src/pages/logistics/LogisticsProductsPage.tsx` `LogisticsMatchRulesPage.tsx`
- 负责人: 已完成

### LOG-002 物流资费维护与估算
- 目标: 物流资费配置并支持发货预估。
- 涉及: 表 `logistics_prices`；API `/logistics/prices` `/logistics/prices/estimate`；UI `/logistics/prices`。
- 分工包: AI-3（物流/打印/售后）
- 验收: 资费配置后可在发货页展示预估。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/logistics/prices` `/logistics/prices/estimate`；`apps/erp-web/src/pages/logistics/LogisticsPricesPage.tsx`
- 负责人: 已完成

## 打印
### PRINT-001 打印执行桥接
- 目标: `print_jobs` 分发到打印桥接服务并回写结果。
- 涉及: `print_jobs`/`print_job_events`；API `/print-jobs/{id}/dispatch`；Worker 打印分发。
- 分工包: AI-3（物流/打印/售后）
- 验收: 打印任务状态可追踪且支持重试。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/print-jobs/{id}/dispatch`；`crates/erp-repo/src/print_jobs.rs` `dispatch_job`；`apps/erp-web/src/pages/printJobs/PrintJobsPage.tsx`
- 负责人: 已完成

### PRINT-002 波次/单据打印联动
- 目标: 波次、出库、发货单可一键生成打印任务。
- 涉及: API `/wms/waves/{id}/print` `/wms/outbounds/{id}/print`；UI 波次/出库/发货页。
- 分工包: AI-3（物流/打印/售后）
- 验收: 打印任务生成并可查看结果。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/wms/waves/{id}/print` `/wms/outbounds/{id}/print`；`apps/erp-web/src/pages/wms/WmsWavesPage.tsx` `WmsOutboundsPage.tsx`
- 负责人: 已完成

## 售后
### AF-001 退货入库回补库存
- 目标: 售后收货回补库存并联动入库。
- 涉及: `return_receipts`/`inventory_ledgers`；API `/after-sales/{id}/putaway`；UI 售后详情。
- 分工包: AI-3（物流/打印/售后）
- 验收: 收货后库存余额变化。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/after-sales/{id}/putaway`；`crates/erp-repo/src/after_sales.rs` `putaway_return_receipt`；`apps/erp-web/src/pages/afterSales/AfterSalesDetailPage.tsx`
- 负责人: 已完成

## 采购与财务
### PROC-001 应付(AP)闭环
- 目标: 采购应付单付款/关单流程。
- 涉及: `ap_docs`；API `/fin/ap-docs/*`；UI `/finance/ap-docs`。
- 分工包: AI-4（采购与财务）
- 验收: 到货验收可生成应付并完成关单。
- 状态: 已完成
- 证据: `crates/erp-repo/src/procurement/store.rs` 应付生成；`crates/erp-api/src/lib.rs`/`crates/erp-api/src/finance_management.rs` 应付与付款计划接口；`apps/erp-web/src/pages/finance/ApDocsPage.tsx`/`ApManagementPage.tsx`
- 负责人: 已完成

### FIN-001 账单导入与支付对账
- 目标: 支付账单导入与对账闭环。
- 涉及: `payment_statements`；API `/fin/statements/import` `/fin/payment-reconcile`；UI `/finance/reconcile`。
- 分工包: AI-4（采购与财务）
- 验收: 导入后可输出对账差异明细。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` 账单导入/支付对账接口；`crates/erp-repo/src/finance.rs` 支付账单与对账存储；`apps/erp-web/src/pages/finance/ReconcilePage.tsx`
- 负责人: 已完成

## 报表与导出
### REP-001 异步导出落地
- 目标: 导出任务生成文件并落地存储/过期清理。
- 涉及: `export_jobs`；Worker `export_jobs`；存储配置（对象存储或挂载目录）。
- 分工包: AI-1（基础账号与系统入口）
- 验收: 导出任务完成后可下载文件。
- 状态: 已完成
- 证据: `crates/erp-api/src/lib.rs` `/exports/jobs`；`crates/erp-worker/src/jobs/export_jobs.rs`；`apps/erp-web/src/pages/exports/ExportJobsPage.tsx`
- 负责人: 已完成

## 可观测性
### OBS-001 可观测性仪表盘/告警
- 目标: 关键链路 metrics/traces 看板与告警。
- 涉及: telemetry exporter、Grafana/Prometheus 配置。
- 分工包: AI-1（基础账号与系统入口）
- 验收: 核心指标有仪表盘与告警阈值。
- 状态: 已完成
- 证据: `apps/erp-web/src/pages/system/ObservabilityPage.tsx`；`apps/erp-web/src/api/system.ts`；`apps/erp-web/src/api/failureQueue.ts`；`crates/erp-api/src/lib.rs`
- 负责人: 已完成
