# Industrial ERP 95%+ Completion Ten-Track Codex Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 以“工业级电商 ERP 首个可收费版本”为目标，将当前项目的整体完成度推进到 95%+，并支持最多 10 个 Codex 子任务并行收口。  
**Architecture:** 采用“十轨并行 + 单主线门禁 + 共享文件隔离 + 最终集成收口”模式。9 个业务/平台轨独占各自文件边界，1 个集成轨负责门禁、证据、CI、文档与最终合并。  
**Tech Stack:** React + TypeScript + Playwright + Rust + Axum + SQLx + PostgreSQL + OpenAPI + pnpm + cargo + Codex worktrees

---

## 0. 执行总则

1. 本计划的完成口径不是“页面数量”，而是“业务闭环 + 持久化证据 + 自动化验证 + 发布门禁”。
2. 每个子任务必须在独立 worktree 和独立分支执行，分支统一使用 `codex/erp95-*` 前缀。
3. 除 `Track-10` 外，禁止跨轨修改 `package.json`、`scripts/`、`docs/ops/`、`.github/workflows/ci.yml`。
4. 除 `Track-01` 外，禁止跨轨修改 `apps/erp-web/src/app/layout/menu.tsx`、`apps/erp-web/src/app/router.tsx`、`apps/erp-web/src/app/layout/routeMeta.ts`。
5. 任一写接口功能完成的定义，必须同时满足：
   - OpenAPI 契约存在且含权限/状态码声明
   - 后端持久化落库可追踪
   - 至少一类自动化测试通过
   - 发布门禁不放过伪通过
6. 任一子任务合并前，必须执行边界校验，禁止越界修改其他轨白名单之外的文件。
7. 10 个子任务全部是“可以独立推进但最终要统一收口”的工作流，允许并行，不允许各自定义完成标准。

---

## 1. 95%+ 完成度判定口径

### 1.1 总目标

- [ ] `docs/spec/erp-launch-full-feature-matrix.md` 中 `P0` 项不再存在 `缺失`
- [ ] 生产菜单可达页面中不再存在“建设中”占位主入口
- [ ] `pnpm run repo:verify` 通过
- [ ] `pnpm run test:backend:release-gate` 通过
- [ ] `pnpm --dir apps/erp-web run generate:api` 通过
- [ ] `pnpm --dir apps/erp-web build` 通过
- [ ] `pnpm run test:e2e:release-gate` 通过
- [ ] OMS、WMS、售后、采购计划、财务、渠道接入六大业务域均有可审计主链路

### 1.2 硬门禁

- [ ] `DATABASE_URL` 或 `TEST_DATABASE_URL` 可连通，且后端关键测试无 `skip`
- [ ] 任一核心链路不能出现“接口成功但无落库证据”
- [ ] `AgGridEnterpriseDemoPage` 不再作为生产菜单入口暴露
- [ ] `SalesEcommercePlaceholderPages.tsx` 对应路由不再作为生产主入口占位
- [ ] 每条主链路至少存在一个后端集成测试和一个前端 E2E 或页面级行为测试

### 1.3 95%+ 解释

本计划将“95%+”定义为：

1. 首发 `P0` 能力全部具备上线证据。
2. 首发范围内 `P1` 能力完成主要闭环，不再只停留在“有页面/有 API”。
3. `P2` 允许保留扩展项，但不得阻断当前商业化上线。

---

## 2. 十轨并行任务图

## 2.1 Track-01 信息架构与生产菜单收口

**Owner:** Codex-01  
**职责:** 完成生产菜单、路由可达性、研发入口隔离、占位页清理。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/moduleSearch.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/moduleSearch.test.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/sales/SalesEcommercePlaceholderPages.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/AgGridEnterpriseDemoPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/frontend-route-observability.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，锁定生产菜单与生产路由可达范围。
- [ ] Step 2: 从生产菜单移除研发演示入口，保留必要的内部调试路由但不暴露。
- [ ] Step 3: 处理销售域占位入口，改为真实页面或临时隐藏，不允许主入口“建设中”。
- [ ] Step 4: 对齐菜单、路由、模块搜索三者的一致性。
- [ ] Step 5: 运行 E2E 与菜单相关单测。
- [ ] Step 6: 更新“生产入口清单”说明并提交。

**验证命令:**

```bash
pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts
pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/frontend-route-observability.spec.ts --project=chromium
```

---

## 2.2 Track-02 多商户与组织权限基线

**Owner:** Codex-02  
**职责:** 为 95% 商业化目标补齐多商户/多组织最小数据模型、权限与页面基线。  
**独占写入范围:**

- Create: `F:/erp/migrations/0095_multi_merchant_org_baseline.sql`
- Modify: `F:/erp/crates/erp-repo/src/tenants.rs`
- Modify: `F:/erp/crates/erp-repo/src/data_scopes.rs`
- Modify: `F:/erp/crates/erp-repo/src/permissions.rs`
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Modify: `F:/erp/crates/erp-api/tests/api_rbac_tenant_isolation.rs`
- Modify: `F:/erp/apps/erp-web/src/pages/system/DataScopesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/UsersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/channels/ShopAccessPage.tsx`

**任务步骤:**

- [ ] Step 1: 先写失败测试，锁定商户/组织/店铺的数据权限行为。
- [ ] Step 2: 补充最小数据库结构与 repo 读写接口。
- [ ] Step 3: 将权限隔离接入 API 层，禁止跨主体越权查询和操作。
- [ ] Step 4: 补充管理页最小展示与编辑能力。
- [ ] Step 5: 运行租户隔离测试与页面回归。
- [ ] Step 6: 输出多商户基线假设与剩余边界说明。

**验证命令:**

```bash
cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture
pnpm --filter erp-web exec playwright test e2e/channels-shops-list-visibility.spec.ts --project=chromium
```

---

## 2.3 Track-03 渠道接入与同步任务运营化

**Owner:** Codex-03  
**职责:** 收口渠道接入、店铺健康、同步任务、失败补偿的运营工作台。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemSyncJobsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/channels/ShopAccessPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/systemIntegrations.ts`
- Modify: `F:/erp/apps/erp-web/src/api/syncJobs.ts`
- Modify: `F:/erp/crates/erp-repo/src/sync_jobs.rs`
- Modify: `F:/erp/crates/erp-repo/src/sync_job_runs.rs`
- Modify: `F:/erp/crates/erp-repo/src/platform_tokens.rs`
- Test: `F:/erp/crates/erp-api/tests/platform_oauth_webhook_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/system_integrations_integration.rs`
- Test: `F:/erp/apps/erp-web/e2e/system-integrations.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，锁定授权到期、同步失败、补偿入口与健康状态。
- [ ] Step 2: 补齐任务运行态与健康摘要接口。
- [ ] Step 3: 补齐前端“失败原因、重试、补偿、最近运行”视图。
- [ ] Step 4: 接入可观测字段和人工处理入口。
- [ ] Step 5: 跑后端集成测试与 E2E。
- [ ] Step 6: 输出接入运营手册的最小字段要求。

**验证命令:**

```bash
cargo test -p erp-api --test platform_oauth_webhook_integration --test system_integrations_integration -- --nocapture
pnpm --filter erp-web exec playwright test e2e/system-integrations.spec.ts --project=chromium
```

---

## 2.4 Track-04 商品主数据与映射治理闭环

**Owner:** Codex-04  
**职责:** 完成商品主档、渠道货品下载、映射治理、变更控制与占位页收口。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductChangeControlPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductChannelAvailabilityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/products.ts`
- Modify: `F:/erp/apps/erp-web/src/api/shopProducts.ts`
- Modify: `F:/erp/crates/erp-repo/src/products.rs`
- Modify: `F:/erp/crates/erp-repo/src/shop_products.rs`
- Modify: `F:/erp/crates/erp-repo/src/channel_sku_mappings.rs`
- Modify: `F:/erp/crates/erp-repo/src/shop_product_tasks.rs`
- Test: `F:/erp/crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`
- Test: `F:/erp/apps/erp-web/e2e/shop-product-match-governance.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/product-change-control.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，锁定下载、匹配、冲突治理、变更审批的最小闭环。
- [ ] Step 2: 把 `ProductDiffPage` 从空壳页推进为可用差异治理页。
- [ ] Step 3: 打通渠道货品下载、匹配任务、映射结果、冲突原因回写。
- [ ] Step 4: 补齐商品变更控制与渠道可售状态的联动展示。
- [ ] Step 5: 跑后端匹配流测试与前端 E2E。
- [ ] Step 6: 记录数据质量规则与人工修复入口。

**验证命令:**

```bash
cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
pnpm --filter erp-web exec playwright test e2e/shop-product-match-governance.spec.ts e2e/product-change-control.spec.ts --project=chromium
```

---

## 2.5 Track-05 销售与 OMS 主链路闭环

**Owner:** Codex-05  
**职责:** 收口审单、异常、拆合单、复核、销售结算和审计落库。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderSettlementPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderSplitMergeLegacyPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts`
- Modify: `F:/erp/crates/erp-api/src/orders.rs`
- Modify: `F:/erp/crates/erp-repo/src/orders.rs`
- Modify: `F:/erp/crates/erp-repo/src/order_reviews.rs`
- Modify: `F:/erp/crates/erp-repo/src/order_action_logs.rs`
- Modify: `F:/erp/crates/erp-repo/src/order_status_history.rs`
- Test: `F:/erp/crates/erp-api/tests/oms_orders.rs`
- Test: `F:/erp/crates/erp-api/tests/order_exception_sla_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/order_match_ship_wave_persistence_guard.rs`
- Test: `F:/erp/apps/erp-web/e2e/order-processing-flow.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/order-exceptions.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，锁定审单动作、异常 SLA、拆合单和结算行为。
- [ ] Step 2: 补齐工具栏动作和异常处理，确保写操作必须落库。
- [ ] Step 3: 完成审单、拆合单、复核、结算的状态链一致性。
- [ ] Step 4: 补齐操作日志与状态历史查询。
- [ ] Step 5: 跑后端集成测试与前端主链路 E2E。
- [ ] Step 6: 记录 OMS 主链路证据表与回归清单。

**验证命令:**

```bash
cargo test -p erp-api --test oms_orders --test order_exception_sla_integration --test order_match_ship_wave_persistence_guard -- --nocapture
pnpm --filter erp-web exec playwright test e2e/order-processing-flow.spec.ts e2e/order-exceptions.spec.ts --project=chromium
```

---

## 2.6 Track-06 仓储、库存与履约一致性

**Owner:** Codex-06  
**职责:** 完成库存承诺、出库、波次、发货确认、库存治理工作台的一致性收口。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx`
- Modify: `F:/erp/crates/erp-repo/src/wms.rs`
- Modify: `F:/erp/crates/erp-repo/src/shipments.rs`
- Modify: `F:/erp/crates/erp-repo/src/inventory.rs`
- Modify: `F:/erp/crates/erp-repo/src/atp_inventory.rs`
- Test: `F:/erp/crates/erp-api/tests/shipment_confirm.rs`
- Test: `F:/erp/crates/erp-api/tests/wave_ship_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/inventory_atp_fefo_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/wms_labor_slotting_flow.rs`
- Test: `F:/erp/apps/erp-web/e2e/wms-wave-main-chain.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，覆盖部分失败、重试、状态回写和 ATP 一致性。
- [ ] Step 2: 收口波次、出库、发货确认和库存治理面板。
- [ ] Step 3: 补齐出库与发货的日志/审计/状态历史。
- [ ] Step 4: 保证库存承诺、批次/FEFO、发货确认的结果一致。
- [ ] Step 5: 跑后端集成测试与 WMS 主链 E2E。
- [ ] Step 6: 输出履约回放与失败补偿说明。

**验证命令:**

```bash
cargo test -p erp-api --test shipment_confirm --test wave_ship_integration --test inventory_atp_fefo_integration --test wms_labor_slotting_flow -- --nocapture
pnpm --filter erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts --project=chromium
```

---

## 2.7 Track-07 采购与计划执行闭环

**Owner:** Codex-07  
**职责:** 打通采购申请、计划、补货建议、采购订单、到货、执行反馈的端到端闭环。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/procurement/ProcurementRequestsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/ProcurementPlansPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/PurchaseOrdersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/PurchaseReceiptsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/PurchaseSuggestionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/ReplenishmentRulesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PurchasePlansPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PlanExecutionPage.tsx`
- Modify: `F:/erp/crates/erp-api/src/planning.rs`
- Modify: `F:/erp/crates/erp-repo/src/procurement.rs`
- Modify: `F:/erp/crates/erp-repo/src/planning.rs`
- Modify: `F:/erp/crates/erp-repo/src/suppliers.rs`
- Test: `F:/erp/crates/erp-api/tests/planning_replenishment_flow.rs`
- Test: `F:/erp/crates/erp-repo/tests/procurement_flow.rs`
- Test: `F:/erp/apps/erp-web/e2e/procurement-pages.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/planning-flow.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，锁定采购和计划的来源、执行、关闭、回写行为。
- [ ] Step 2: 补齐采购建议、补货计划、采购订单和到货之间的数据衔接。
- [ ] Step 3: 在页面中提供执行反馈与异常说明。
- [ ] Step 4: 补齐供应商和计划参数的最小审计信息。
- [ ] Step 5: 跑后端与前端回归。
- [ ] Step 6: 输出采购计划闭环的 DoD 证据。

**验证命令:**

```bash
cargo test -p erp-api --test planning_replenishment_flow -- --nocapture
cargo test -p erp-repo --test procurement_flow -- --nocapture
pnpm --filter erp-web exec playwright test e2e/procurement-pages.spec.ts e2e/planning-flow.spec.ts --project=chromium
```

---

## 2.8 Track-08 客户、售后与逆向履约

**Owner:** Codex-08  
**职责:** 完成售后主链、客服工单、SLA、逆向收货/质检/补发和平台售后聚合闭环。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesExchangesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesInspectionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/RefundDocsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/customers/CustomerServiceTicketsPage.tsx`
- Modify: `F:/erp/crates/erp-repo/src/after_sales.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sales_sla_rules.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sales_quality_rules.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sales_intercepts.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sale_cases.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_state_machine_guard.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_sla_escalation.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_exchange_profit_flow.rs`
- Test: `F:/erp/apps/erp-web/e2e/after-sales-workflow.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/customer-service-tickets.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，覆盖售后状态机、逆向收货、质检、补发、SLA 和工单联动。
- [ ] Step 2: 收口售后列表、SLA 和工单视角的责任归因。
- [ ] Step 3: 补齐退款单据、补发与平台售后聚合的日志证据。
- [ ] Step 4: 确保客服催办与 SLA 字段一致。
- [ ] Step 5: 跑后端测试与售后 E2E。
- [ ] Step 6: 更新售后上线门禁勾选状态。

**验证命令:**

```bash
cargo test -p erp-api --test after_sales --test after_sales_state_machine_guard --test after_sales_sla_escalation --test after_sales_exchange_profit_flow -- --nocapture
pnpm --filter erp-web exec playwright test e2e/after-sales-workflow.spec.ts e2e/after-sales-return-resend-main-chain.spec.ts e2e/customer-service-tickets.spec.ts --project=chromium
```

---

## 2.9 Track-09 财务、对账、发票与经营分析

**Owner:** Codex-09  
**职责:** 收口应收应付、对账中心、费用分摊、利润报表、发票与售后财务联动。  
**独占写入范围:**

- Modify: `F:/erp/apps/erp-web/src/pages/finance/ArManagementPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ApManagementPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ReconcilePage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ExpenseAllocationPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ProfitPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/InvoicesPage.tsx`
- Modify: `F:/erp/crates/erp-api/src/finance_management.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_ar.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_ap_plans.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_expenses.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_costing.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_tax.rs`
- Test: `F:/erp/crates/erp-api/tests/finance_reconcile.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_finance_reconcile.rs`
- Test: `F:/erp/crates/erp-repo/tests/finance_profit.rs`
- Test: `F:/erp/apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts`

**任务步骤:**

- [ ] Step 1: 写失败测试，锁定 AR/AP/对账/费用/利润/开票的最小闭环。
- [ ] Step 2: 补齐财务视图与业务单据的关联字段。
- [ ] Step 3: 补齐售后退款、费用分摊、利润口径的联动。
- [ ] Step 4: 在前端输出差异原因、状态和审计证据。
- [ ] Step 5: 跑财务集成测试与经营分析 E2E。
- [ ] Step 6: 输出财务首发范围与剩余增强项说明。

**验证命令:**

```bash
cargo test -p erp-api --test finance_reconcile --test after_sales_finance_reconcile -- --nocapture
cargo test -p erp-repo --test finance_profit -- --nocapture
pnpm --filter erp-web exec playwright test e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium
```

---

## 2.10 Track-10 集成收口、门禁、CI 与证据归档

**Owner:** Codex-10（集成负责人）  
**职责:** 统一收口 01-09 轨产出，维护发布门禁、CI、任务卡、观测证据和最终评分。  
**独占写入范围:**

- Modify: `F:/erp/package.json`
- Modify: `F:/erp/.github/workflows/ci.yml`
- Modify: `F:/erp/scripts/release_gate_backend.mjs`
- Modify: `F:/erp/scripts/verify_openapi_gate_v2.mjs`
- Modify: `F:/erp/scripts/verify_repo_baseline.mjs`
- Modify: `F:/erp/docs/ops/RELEASE_GATE_CHECKLIST_V2.md`
- Modify: `F:/erp/docs/ops/GO_NO_GO_CHECKLIST_V2.md`
- Modify: `F:/erp/docs/ops/TASK_COMPLETION_MATRIX_V2.md`
- Modify: `F:/erp/docs/ops/progress_check.md`
- Modify: `F:/erp/docs/reports/更新说明_2026-04-14.md`
- Modify: `F:/erp/apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/frontendRouteObservability.ts`
- Modify: `F:/erp/infra/observability/prometheus/alerts.yml`
- Modify: `F:/erp/docs/spec/erp-launch-full-feature-matrix.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-01.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-02.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-03.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-04.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-05.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-06.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-07.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-08.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-09.md`
- Create: `F:/erp/docs/ops/tasks/2026-04-14/erp95-track-10.md`

**任务步骤:**

- [ ] Step 1: 为 01-10 轨生成任务卡，并登记白名单与禁改文件。
- [ ] Step 2: 将所有新增能力纳入 `repo:verify`、后端 release gate、前端 release gate。
- [ ] Step 3: 补齐可观测与告警项，确保核心指标可见。
- [ ] Step 4: 更新功能矩阵，把“已具备/部分具备/缺失”状态改为最新结论。
- [ ] Step 5: 在真实数据库环境跑通全量门禁并沉淀证据。
- [ ] Step 6: 生成阶段评分卡和 Go/No-Go 结论。
- [ ] Step 7: 统一合并、冲突收敛、回归验证并提交最终集成分支。

**验证命令:**

```bash
pnpm run repo:verify
pnpm run test:backend:release-gate
pnpm --dir apps/erp-web run generate:api
pnpm --dir apps/erp-web build
pnpm run test:e2e:release-gate
```

---

## 3. 并行启动命令

```bash
git fetch origin
git worktree add .worktrees/erp95-01 -b codex/erp95-01-ia origin/main
git worktree add .worktrees/erp95-02 -b codex/erp95-02-merchant-org origin/main
git worktree add .worktrees/erp95-03 -b codex/erp95-03-channel-ops origin/main
git worktree add .worktrees/erp95-04 -b codex/erp95-04-product-governance origin/main
git worktree add .worktrees/erp95-05 -b codex/erp95-05-oms origin/main
git worktree add .worktrees/erp95-06 -b codex/erp95-06-wms origin/main
git worktree add .worktrees/erp95-07 -b codex/erp95-07-procurement-planning origin/main
git worktree add .worktrees/erp95-08 -b codex/erp95-08-aftersales origin/main
git worktree add .worktrees/erp95-09 -b codex/erp95-09-finance origin/main
git worktree add .worktrees/erp95-10 -b codex/erp95-10-integration-gate origin/main
```

---

## 4. 任务卡落地规则

1. `Track-10` 负责先为每个轨生成任务卡，输出到 `docs/ops/tasks/2026-04-14/erp95-track-*.md`。
2. 每个任务卡都必须包含：
   - `branch`
   - `worktree`
   - `allowed_files`
   - `disallowed_files`
   - `dod`
   - `test_commands`
3. 每次提交前必须执行边界校验：

```bash
node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/erp95-track-05.md --mode working
```

4. 每个轨交付格式统一为：
   - `commit hash`
   - 变更摘要
   - 测试结果
   - 残余风险

---

## 5. 合并顺序

1. `Track-02` 多商户与组织权限基线
2. `Track-03` 渠道接入与同步任务运营化
3. `Track-04` 商品主数据与映射治理闭环
4. `Track-05` 销售与 OMS 主链路闭环
5. `Track-06` 仓储、库存与履约一致性
6. `Track-07` 采购与计划执行闭环
7. `Track-08` 客户、售后与逆向履约
8. `Track-09` 财务、对账、发票与经营分析
9. `Track-01` 信息架构与生产菜单收口
10. `Track-10` 集成收口、门禁、CI 与证据归档

说明：

- `Track-01` 放在靠后阶段，是为了避免前期菜单/路由频繁冲突。
- `Track-10` 永远最后合并，只做集成和门禁，不承担大块业务实现。

---

## 6. 统一 DoD

### 6.1 业务 DoD

- [ ] 生产菜单可见的每个首发入口都能完成真实业务动作，不是占位页。
- [ ] 渠道、商品、OMS、WMS、采购计划、售后、财务七大域都有至少一条主链路可跑通。
- [ ] 关键业务写操作都有落库证据与审计记录。

### 6.2 测试 DoD

- [ ] 每个轨至少新增或补强一个后端测试。
- [ ] 每个前端业务轨至少新增或补强一个 E2E。
- [ ] `pnpm run release:gate:full` 在 CI 或高资源环境通过。

### 6.3 文档 DoD

- [ ] `docs/spec/erp-launch-full-feature-matrix.md` 更新为最新状态。
- [ ] `docs/ops/RELEASE_GATE_CHECKLIST_V2.md` 与脚本行为一致。
- [ ] `docs/ops/GO_NO_GO_CHECKLIST_V2.md` 可用于最终签署。
- [ ] 10 个轨的任务卡、测试命令、风险说明齐全。

---

## 7. 风险与止损

- [ ] 若 `Track-02` 结构改动过大：先落最小字段和权限隔离，不在本轮展开复杂法人结算。
- [ ] 若 `Track-04` 映射治理延期：优先保证手工匹配、冲突原因、任务日志可追踪。
- [ ] 若 `Track-06` 履约链路复杂度过高：先确保出库、波次、发货确认的状态一致，不扩展设备集成。
- [ ] 若 `Track-09` 财务深度不足：先保证对账中心、利润报表、售后财务回写三条主线，不展开总账。
- [ ] 若本机资源导致后端门禁不稳定：必须在 CI 或高资源机器重跑，不允许以环境借口签署 Go。

---

## 8. 最终交付物

- [ ] 10 条并行子任务卡
- [ ] 10 条独立分支或 worktree 交付记录
- [ ] 全量门禁通过证据
- [ ] 更新后的能力矩阵
- [ ] 最终 Go/No-Go 结论
- [ ] 工业级 95%+ 完成度说明
