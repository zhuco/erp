# ERP 功能矩阵（全盘盘点）

> 状态声明（2026-04-10）：本文件为阶段性盘点快照，存在与当前实现进度不一致的条目。  
> 当前功能完成口径请优先参考 `docs/ops/TASK_COMPLETION_MATRIX.md` 与 `docs/ops/RELEASE_GATE.md`。  
> 若与 OpenAPI/代码实现冲突，以 `openapi/erp_api_v1.yaml`、`crates/*`、`apps/erp-web/*` 为准。

数据来源：`README.md`、`docs/spec/*`、`openapi/erp_api_v1.yaml`、`migrations/*`、`apps/erp-web` 路由与菜单、`crates/*` 关键实现。
最后更新：2026-01-17
状态说明：已实现 / 部分 / 缺失（“部分”代表已有数据结构/接口/代码，但缺 UI、自动化、或未闭环）
备注：本矩阵不覆盖平台对外接入（OAuth/验签/拉单/回传/库存回写）。

## SaaS 底座
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| SaaS 底座 | Tenant/Shop 管理 | 已实现 | SQL: `migrations/0001_init.sql` (tenants/shops)<br>API: `/v1/tenants/{tenant_id}/shops` in `openapi/erp_api_v1.yaml`<br>Rust: `crates/erp-api/src/lib.rs` `list_shops`/`create_shop`<br>UI: `apps/erp-web/src/app/router.tsx` `settings/shops` | 多租户字段已覆盖 |
| SaaS 底座 | RBAC/JWT + 权限目录 | 已实现 | SQL: `migrations/0001_init.sql` (users/roles/role_permissions)<br>SQL: `migrations/0040_permissions_catalog.sql`<br>API: `/v1/tenants/{tenant_id}/roles` `/users` `/permissions`<br>Rust: `crates/erp-api/src/lib.rs` `ensure_permission`<br>UI: `/system/roles` `/system/users` `/system/permissions` | 权限粒度可继续细化 |
| SaaS 底座 | 数据权限（店铺/仓库） | 已实现 | SQL: `migrations/0030_system_management.sql` (user_shop_access/user_warehouse_access)<br>API: `/v1/tenants/{tenant_id}/users/{user_id}/data-scopes`<br>Rust: `crates/erp-api/src/lib.rs` `ensure_warehouse_access`<br>UI: `/system/data-scopes` | 需持续校验接口层覆盖 |
| SaaS 底座 | 审计/操作日志 | 已实现 | SQL: `migrations/0030_system_management.sql` (audit_logs)<br>SQL: `migrations/0017_operation_logs.sql` (operation_logs)<br>API: `/v1/tenants/{tenant_id}/audit-logs` `/operation-logs`<br>UI: `/system/audit-logs` `/system/operation-logs` | 需补日志留存策略 |
| SaaS 底座 | 配置中心/特性开关 | 已实现 | SQL: `migrations/0030_system_management.sql` (settings)<br>SQL: `migrations/0015_feature_flags.sql`<br>API: `/v1/tenants/{tenant_id}/settings` `/feature-flags`<br>UI: `/settings` `/system/flow-switches` | 配置项需扩展 |
| SaaS 底座 | DLQ/Outbox/Inbox 可靠性 | 已实现 | SQL: `migrations/0001_init.sql` (inbox/outbox)<br>SQL: `migrations/0012_dead_letters.sql` `migrations/0034_event_reliability.sql`<br>API: `/v1/tenants/{tenant_id}/dead-letters` `/outbox-events`<br>Rust: `crates/erp-worker/src/main.rs` `handle_dead_letter_replay`<br>UI: `/failure-queue` | 建议补自动重试任务 |
| SaaS 底座 | 可观测性 | 已实现 | Docs: `docs/ops/OBSERVABILITY.md`<br>Rust: `crates/erp-common/src/telemetry.rs`<br>API: `/metrics` `/v1/tenants/{tenant_id}/dead-letters` `/outbox-events`<br>UI: `/system/observability` | 告警阈值与通知已补齐 |

## OMS
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| OMS | 订单投影查询 | 已实现 | SQL: `migrations/0001_init.sql` (orders/order_items)<br>API: `/v1/tenants/{tenant_id}/orders` `/orders/{order_id}`<br>Rust: `crates/erp-api/src/lib.rs` `list_orders`/`get_order_detail`<br>UI: `/orders` | 列表分页已具备 |
| OMS | 审单/复核 | 部分 | SQL: `migrations/0020_oms_orders_review.sql` (order_review_rules/order_review_hits)<br>SQL: `migrations/0030_system_management.sql` (order_reviews)<br>API: `/v1/tenants/{tenant_id}/oms/review-rules` `/orders/{order_id}/review`<br>UI: `/orders/review` `/orders/review-rules` | 规则命中闭环需打通 |
| OMS | 异常池/风控 | 部分 | SQL: `migrations/0020_oms_orders_review.sql` (exception_flags/exception_reasons/customer_blacklist)<br>API: `/orders/{order_id}/exceptions/handle` `/oms/exception-reasons` `/oms/customer-blacklist`<br>UI: `/orders/review`（异常池入口） | 缺异常处理流程与统计 |
| OMS | 标签/备注/自动标签 | 已实现 | SQL: `migrations/0020_oms_orders_review.sql` (order_tags/order_notes)<br>API: `/orders/{order_id}/tags` `/orders/{order_id}/notes` `/orders/auto-tags/apply`<br>UI: 订单详情页标签/备注 | 需完善批量操作 |
| OMS | 订单分仓/发货计划 | 部分 | SQL: `migrations/0022_oms_fulfillment.sql` (warehouse_rules/shipment_items)<br>API: `/v1/tenants/{tenant_id}/oms/orders/{order_id}/allocate` `/oms/shipments`<br>UI: `/oms/shipments` | 分仓策略需优化 |
| OMS | 多包裹 | 部分 | SQL: `migrations/0032_wms_outbound.sql` (pack_records)<br>API: `/v1/tenants/{tenant_id}/wms/outbounds/{id}/pack`<br>UI: `/wms/pack` | OMS 回传链路尚未完全打通 |

## 商品
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 商品 | SPU/商品档案 | 已实现 | SQL: `migrations/0004_product_sku_schema.sql` (products)<br>API: `/v1/tenants/{tenant_id}/products`<br>UI: `/products` | 基础字段齐备 |
| 商品 | SKU/条码 | 已实现 | SQL: `migrations/0004_product_sku_schema.sql` (skus/barcode idx)<br>API: `/v1/tenants/{tenant_id}/skus`<br>UI: `/skus` | |
| 商品 | 渠道 SKU 映射 | 已实现 | SQL: `migrations/0004_product_sku_schema.sql` (channel_sku_mappings)<br>API: `/v1/tenants/{tenant_id}/channel-sku-mappings`<br>UI: `/mappings` | |
| 商品 | 组合包/BOM | 已实现 | SQL: `migrations/0006_kit_bom.sql` (kit_boms)<br>API: `/v1/tenants/{tenant_id}/kits`<br>UI: `/kits` | |
| 商品 | 批次管理 | 已实现 | SQL: `migrations/0029_product_center.sql` (sku_batches)<br>API: `/v1/tenants/{tenant_id}/sku-batches`<br>UI: `/sku-batches` | 需补效期策略 |
| 商品 | SN/序列号 | 已实现 | SQL: `migrations/0029_product_center.sql` (sku_serials)<br>API: `/v1/tenants/{tenant_id}/sku-serials` `/sku-serials/print`<br>UI: `/sku-serials` | |

## 库存
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 库存 | 余额/占用/流水 | 已实现 | SQL: `migrations/0005_inventory_schema.sql` (inventory_balances/ledgers/reservations)<br>API: `/v1/tenants/{tenant_id}/inventory` `/inventory/ledgers`<br>UI: `/inventory` `/inventory/ledgers` | |
| 库存 | 调整/预留/释放 | 已实现 | API: `/v1/tenants/{tenant_id}/inventory/adjustments` `/reservations` `/releases`<br>Rust: `crates/erp-api/src/lib.rs` `apply_inventory_movement`<br>UI: `/wms/stock-adjustments` | |
| 库存 | 冻结 | 已实现 | SQL: `migrations/0031_wms_inventory_ops.sql` (inventory_freezes)<br>API: `/v1/tenants/{tenant_id}/inventory/freezes`<br>UI: `/wms/inventory-freezes` | |
| 库存 | 库存幂等保障 | 已实现 | SQL: `migrations/0009_inventory_return_idempotency.sql`<br>Rust: `crates/erp-repo/src/inventory.rs` (outbox 去重) | 需持续压测 |

## WMS
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| WMS | 入库/收货/上架 | 已实现 | SQL: `migrations/0021_wms_inbounds.sql`<br>API: `/v1/tenants/{tenant_id}/wms/inbounds` `/receive` `/putaway`<br>UI: `/wms/inbounds` | |
| WMS | 出库/波次/拣货/打包/发货 | 已实现 | SQL: `migrations/0032_wms_outbound.sql`<br>API: `/v1/tenants/{tenant_id}/wms/outbounds` `/waves` `/pick-tasks` `/pack` `/ship`<br>UI: `/wms/outbounds` `/wms/waves` `/wms/pick-tasks` `/wms/pack` `/wms/ship` | |
| WMS | 复核/称重 | 已实现 | SQL: `migrations/0060_wms_pack_checks.sql` (pack_check_events)<br>API: `/v1/tenants/{tenant_id}/wms/outbounds/{outbound_doc_id}/check` `/weigh`<br>UI: `apps/erp-web/src/pages/wms/WmsPackPage.tsx` | |
| WMS | 盘点 | 已实现 | SQL: `migrations/0031_wms_inventory_ops.sql` (stock_counts)<br>API: `/v1/tenants/{tenant_id}/stock-counts*`<br>UI: `/wms/stock-counts` | |
| WMS | 调拨/移库 | 已实现 | SQL: `migrations/0031_wms_inventory_ops.sql` (stock_transfers/stock_moves)<br>API: `/v1/tenants/{tenant_id}/stock-transfers*` `/stock-moves*`<br>UI: `/wms/stock-transfers` `/wms/stock-moves` | |

## 售后
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 售后 | 售后单管理 | 已实现 | SQL: `migrations/0023_after_sales_module.sql` (after_sales/after_sales_items)<br>API: `/v1/tenants/{tenant_id}/after-sales`<br>UI: `/after-sales` | |
| 售后 | AfterSale Case 聚合 | 已实现 | SQL: `migrations/0003_after_sale_cases.sql`<br>API: `/v1/tenants/{tenant_id}/after-sale-cases`<br>UI: `/after-sale-cases` | |
| 售后 | 退款单 | 已实现 | SQL: `migrations/0023_after_sales_module.sql` (refund_docs)<br>API: `/v1/tenants/{tenant_id}/refund-docs`<br>UI: `/refund-docs` | |
| 售后 | 退货入库回补库存 | 已实现 | SQL: `migrations/0023_after_sales_module.sql` (return_receipts)<br>API: `/v1/tenants/{tenant_id}/after-sales/{id}/putaway`<br>UI: `apps/erp-web/src/pages/afterSales/AfterSalesDetailPage.tsx` | |
| 售后 | 换货/补发 | 部分 | SQL: `migrations/0023_after_sales_module.sql` (after_sales.type)<br>API/UI: 未见专用流程 | 需补业务流程 |

## 采购
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 采购 | 供应商管理 | 已实现 | SQL: `migrations/0033_procurement.sql` (suppliers)<br>API: `/v1/tenants/{tenant_id}/suppliers`<br>UI: `/suppliers` | |
| 采购 | 采购单/到货验收 | 已实现 | SQL: `migrations/0033_procurement.sql` (purchase_orders/purchase_receipts)<br>API: `/v1/tenants/{tenant_id}/purchase-orders` `/receive`<br>UI: `/procurement/orders` | |
| 采购 | 采购退货 | 已实现 | SQL: `migrations/0033_procurement.sql` (purchase_returns)<br>API: `/v1/tenants/{tenant_id}/purchase-returns`<br>UI: `/procurement/returns` | |
| 采购 | 采购建议 | 已实现 | SQL: `migrations/0033_procurement.sql` (purchase_suggestions)<br>API: `/v1/tenants/{tenant_id}/purchase-suggestions`<br>UI: `/procurement/suggestions` | |
| 采购 | 应付/AP | 已实现 | SQL: `migrations/0033_procurement.sql` (ap_docs)<br>API: `/v1/tenants/{tenant_id}/fin/ap-docs`<br>UI: `/finance/ap-docs` | 付款/关单链路已补齐 |

## 财务对账
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 财务对账 | 账单/快递对账 | 已实现 | SQL: `migrations/0035_finance_reconcile.sql` (platform_statements/shipping_statement_lines/reconcile_jobs)<br>API: `/v1/tenants/{tenant_id}/fin/reconcile/run` `/fin/reconcile/jobs`<br>UI: `/finance/reconcile` | 支持账单导入与对账 |
| 财务对账 | 支付对账 | 已实现 | API: `/v1/tenants/{tenant_id}/fin/payment-reconcile*`<br>UI: `/finance/reconcile` | 自动化对账与差异明细 |
| 财务对账 | 发票 | 已实现 | SQL: `migrations/0035_finance_reconcile.sql` (invoices)<br>API: `/v1/tenants/{tenant_id}/fin/invoices` `/fin/invoices/apply`<br>UI: `/finance/invoices` | |
| 财务对账 | 利润快照 | 已实现 | SQL: `migrations/0035_finance_reconcile.sql` (profit_snapshots)<br>API: `/v1/tenants/{tenant_id}/fin/profit`<br>UI: `/finance/profit` | 需完善成本口径 |

## 打印物流
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 打印物流 | 打印模板/任务/设备 | 已实现 | SQL: `migrations/0008_printing_schema.sql` `migrations/0046_print_device_token.sql`<br>API: `/v1/tenants/{tenant_id}/print-templates` `/print-jobs` `/print-devices`<br>UI: `/print-templates` `/print-jobs` | |
| 打印物流 | 承运商/面单账号 | 已实现 | SQL: `migrations/0037_printing_logistics.sql` (carriers/shop_carrier_accounts)<br>API: `/v1/tenants/{tenant_id}/carriers` `/shop-carrier-accounts`<br>UI: `/couriers` | |
| 打印物流 | 物流轨迹/面单分配 | 已实现 | SQL: `migrations/0007_shipping_schema.sql` + `migrations/0037_printing_logistics.sql` (shipment_tracking_events)<br>API: `/v1/tenants/{tenant_id}/logistics/shipments/*` `/waybills/allocate`<br>UI: `/logistics/tracking` `/shipments` | |
| 打印物流 | 打印执行桥接 | 已实现 | Rust: `crates/erp-print-agent/src/main.rs`<br>API: `/v1/print-agent/jobs` `/v1/tenants/{tenant_id}/print-jobs/{id}/dispatch` in `openapi/erp_api_v1.yaml`<br>UI: `apps/erp-web/src/pages/printJobs/PrintJobsPage.tsx` | |

## 报表导出
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 报表导出 | 经营/履约/库存/财务报表 | 已实现 | API: `/reports/dashboard` `/reports/fulfillment` `/reports/inventory` `/reports/finance`<br>Rust: `crates/erp-api/src/lib.rs` `get_*_report`<br>UI: `/reports` | |
| 报表导出 | 异步导出任务 | 已实现 | SQL: `migrations/0038_reports_exports.sql` (export_jobs)<br>API: `/exports/jobs` `/exports/jobs/{id}/file`<br>UI: `/exports` | |
| 报表导出 | 导出 Worker 生成文件 | 已实现 | Rust: `crates/erp-worker/src/jobs/export_jobs.rs`<br>SQL: `migrations/0038_reports_exports.sql` (output_url) | 输出存储策略见 `docs/ops/DATA_RETENTION.md` |

## 客户/会员
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 客户/会员 | 客户档案 | 已实现 | SQL: `migrations/0050_customers.sql` (customers)<br>API: `/v1/tenants/{tenant_id}/customers`<br>UI: `/customers` `/customers/:customer_id` | CRM lite 形态 |
| 客户/会员 | 客户分类 | 已实现 | SQL: `migrations/0050_customers.sql` (customer_categories)<br>API: `/v1/tenants/{tenant_id}/customer-categories`<br>UI: `/customers/categories` | |
| 客户/会员 | 会员等级 | 已实现 | SQL: `migrations/0050_customers.sql` (member_levels)<br>API: `/v1/tenants/{tenant_id}/member-levels`<br>UI: `/customers/member-levels` | |
| 客户/会员 | 客户标签 | 已实现 | SQL: `migrations/0050_customers.sql` (customer_tags/customer_tag_links)<br>API: `/v1/tenants/{tenant_id}/customer-tags` `/customers/{customer_id}/tags`<br>UI: `/customers/tags` | |

## 营销中心
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 营销中心 | 价目表/价格体系 | 已实现 | SQL: `migrations/0073_marketing_management.sql` (marketing_price_lists/items)<br>Rust: `crates/erp-api/src/marketing.rs`<br>UI: `/marketing/prices` | OpenAPI 未覆盖营销接口 |
| 营销中心 | 促销活动 | 已实现 | SQL: `migrations/0073_marketing_management.sql` (marketing_promotions)<br>Rust: `crates/erp-api/src/marketing.rs`<br>UI: `/marketing/promotions` | OpenAPI 未覆盖 |
| 营销中心 | 赠品策略 | 已实现 | SQL: `migrations/0073_marketing_management.sql` (marketing_gift_policies)<br>Rust: `crates/erp-api/src/marketing.rs`<br>UI: `/marketing/gifts` | OpenAPI 未覆盖 |
| 营销中心 | 优惠券 | 已实现 | SQL: `migrations/0073_marketing_management.sql` (marketing_coupons)<br>Rust: `crates/erp-api/src/marketing.rs`<br>UI: `/marketing/coupons` | OpenAPI 未覆盖 |
| 营销中心 | 促销费用 | 已实现 | SQL: `migrations/0073_marketing_management.sql` (marketing_costs)<br>Rust: `crates/erp-api/src/marketing.rs`<br>UI: `/marketing/costs` | OpenAPI 未覆盖 |
| 营销中心 | 活动复盘 | 已实现 | SQL: `migrations/0073_marketing_management.sql` (marketing_postmortems)<br>Rust: `crates/erp-api/src/marketing.rs`<br>UI: `/marketing/postmortems` | OpenAPI 未覆盖 |

## 规划与预测
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 规划 | 销售预测/补货规则 | 占位 | UI: `apps/erp-web/src/pages/planning/SalesForecastPage.tsx`<br>`apps/erp-web/src/pages/planning/ReplenishmentRulesPage.tsx` | 静态数据与演示逻辑 |
| 规划 | 采购计划/调拨建议 | 占位 | UI: `apps/erp-web/src/pages/planning/PurchasePlansPage.tsx`<br>`TransferSuggestionsPage.tsx` | 无后端接口 |
| 规划 | 促销备货/执行看板 | 占位 | UI: `apps/erp-web/src/pages/planning/PromoStockingPage.tsx`<br>`PlanExecutionPage.tsx` | 无后端接口 |

## 统计与看板
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 统计 | 发货/销售/趋势分析 | 占位 | UI: `apps/erp-web/src/pages/stats/StatsPlaceholder.tsx`<br>路由: `/stats/*` | 等待指标接口 |

## 系统扩展配置
| 模块 | 子功能 | 状态 | 证据路径 | 备注/风险 |
| --- | --- | --- | --- | --- |
| 系统扩展 | 单据编号与流程配置 | 占位 | UI: `apps/erp-web/src/pages/system/SystemWorkflowsPage.tsx` | 本地存储/静态数据 |
| 系统扩展 | 自定义字段 | 占位 | UI: `apps/erp-web/src/pages/system/SystemCustomFieldsPage.tsx` | 本地存储/静态数据 |
| 系统扩展 | 集成中心 | 占位 | UI: `apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx` | 本地存储/静态数据 |
| 系统扩展 | 审计报表 | 占位 | UI: `apps/erp-web/src/pages/system/SystemAuditReportsPage.tsx` | 本地存储/静态数据 |
