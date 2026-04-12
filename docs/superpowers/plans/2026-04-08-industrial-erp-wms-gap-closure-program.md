# 工业级电商 ERP/WMS 缺口收口 Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不推翻当前仓库主栈的前提下，把现有 ERP/WMS 升级为工业级电商仓配系统。

**Architecture:** 按“平台与扩展基础 -> OMS/WMS 工业化 -> 计划补货与利润经营 -> 数据产品与运维硬化”推进。核心交易仍留在 `Rust + Postgres + NATS`，流程编排引入 `Camunda 8`，专题分析引入 `ClickHouse`，统一身份引入 `Keycloak`。

**Tech Stack:** Rust、SQLx、Postgres、NATS、OpenAPI、React、Vite、Ant Design、Playwright、Camunda 8、ClickHouse、Keycloak

---

## 前置说明

- 当前目录 `F:/erp` 不是 Git 工作树，以下 `git commit` 步骤需要在真实 Git 工作树中执行。
- 本计划是总计划，建议一条轨道一条轨道执行，不建议一次性并发推进全部工作流。

### Task 1: 平台接入生产化与集成中心

**Files:**
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Modify: `F:/erp/crates/erp-channel/src/main.rs`
- Modify: `F:/erp/crates/erp-worker/src/main.rs`
- Modify: `F:/erp/crates/erp-repo/src/platform_tokens.rs`
- Modify: `F:/erp/crates/erp-repo/src/settings.rs`
- Modify: `F:/erp/openapi/erp_api_v1.yaml`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Create: `F:/erp/crates/erp-api/tests/platform_oauth_webhook_integration.rs`
- Create: `F:/erp/apps/erp-web/e2e/system-integrations.spec.ts`

- [ ] **Step 1: 写失败的 OAuth、Webhook、字段映射流程测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture`，确认失败**
- [ ] **Step 3: 落地真实凭证、严格验签、回放、同步 SLA 和集成中心持久化**
- [ ] **Step 4: 重跑后端测试与 `pnpm --dir apps/erp-web test:e2e -- e2e/system-integrations.spec.ts`**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-api/src/lib.rs crates/erp-channel/src/main.rs crates/erp-worker/src/main.rs crates/erp-repo/src/platform_tokens.rs crates/erp-repo/src/settings.rs openapi/erp_api_v1.yaml apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx crates/erp-api/tests/platform_oauth_webhook_integration.rs apps/erp-web/e2e/system-integrations.spec.ts
git commit -m "feat: industrialize platform integration center"
```

### Task 2: OMS 工业化与订单异常 SLA 中台

**Files:**
- Modify: `F:/erp/crates/erp-api/src/orders.rs`
- Modify: `F:/erp/crates/erp-repo/src/orders/repo.rs`
- Modify: `F:/erp/crates/erp-repo/src/order_reviews.rs`
- Modify: `F:/erp/crates/erp-repo/src/order_review_rules.rs`
- Modify: `F:/erp/crates/erp-repo/src/order_status_history.rs`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/orders/OrderSettlementPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/orders/OrderSplitMergePage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/orders/OrderReconciliationsPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/orders/OrderExceptionSlaPage.tsx`
- Create: `F:/erp/crates/erp-api/tests/order_exception_sla_integration.rs`

- [ ] **Step 1: 写失败的异常 SLA、拆合单、回退下载集成测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test order_exception_sla_integration -- --nocapture`，确认失败**
- [ ] **Step 3: 落地审单台、异常池、结算/对账台、拆合单与状态轨迹**
- [ ] **Step 4: 重跑后端测试与 OMS 页面 E2E**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-api/src/orders.rs crates/erp-repo/src/orders crates/erp-repo/src/order_reviews.rs crates/erp-repo/src/order_review_rules.rs crates/erp-repo/src/order_status_history.rs apps/erp-web/src/pages/orders crates/erp-api/tests/order_exception_sla_integration.rs
git commit -m "feat: industrialize oms exception and settlement workbench"
```

### Task 3: WMS 现场执行、labor、slotting、补货与月台

**Files:**
- Modify: `F:/erp/crates/erp-repo/src/wms/repo.rs`
- Modify: `F:/erp/crates/erp-repo/src/wms/store.rs`
- Create: `F:/erp/crates/erp-repo/src/wms_labor.rs`
- Create: `F:/erp/crates/erp-repo/src/wms_slotting.rs`
- Create: `F:/erp/crates/erp-repo/src/wms_replenishment.rs`
- Create: `F:/erp/crates/erp-repo/src/wms_dock_yard.rs`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsPackPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/wms/mobile/PdaReceivePage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/wms/mobile/PdaPutawayPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/wms/mobile/PdaPickPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/wms/mobile/PdaCountPage.tsx`
- Create: `F:/erp/crates/erp-api/tests/wms_labor_slotting_flow.rs`

- [ ] **Step 1: 写失败的补货、slotting、PDA、交接台流程测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture`，确认失败**
- [ ] **Step 3: 落地现场执行层，不再只做后台列表页**
- [ ] **Step 4: 重跑 WMS 测试与 `e2e/wms-wave-main-chain.spec.ts`、`e2e/wms-handover.spec.ts`**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-repo/src/wms crates/erp-repo/src/wms_labor.rs crates/erp-repo/src/wms_slotting.rs crates/erp-repo/src/wms_replenishment.rs crates/erp-repo/src/wms_dock_yard.rs apps/erp-web/src/pages/wms apps/erp-web/src/pages/wms/mobile crates/erp-api/tests/wms_labor_slotting_flow.rs
git commit -m "feat: add industrial wms floor execution modules"
```

### Task 4: 库存可承诺、效期与主数据治理

**Files:**
- Modify: `F:/erp/crates/erp-repo/src/inventory.rs`
- Modify: `F:/erp/crates/erp-repo/src/products.rs`
- Modify: `F:/erp/crates/erp-repo/src/skus.rs`
- Modify: `F:/erp/crates/erp-repo/src/sku_batches.rs`
- Create: `F:/erp/crates/erp-repo/src/atp_inventory.rs`
- Create: `F:/erp/crates/erp-repo/src/product_packaging_specs.rs`
- Create: `F:/erp/crates/erp-repo/src/inventory_reconciliation_center.rs`
- Modify: `F:/erp/apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/products/ProductPackagingSpecsPage.tsx`
- Create: `F:/erp/crates/erp-api/tests/inventory_atp_fefo_integration.rs`

- [ ] **Step 1: 写失败的 ATP、FEFO、包装层级与库存对账测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture`，确认失败**
- [ ] **Step 3: 落地 ATP、效期分配、包装规格与库存对账中心**
- [ ] **Step 4: 重跑集成测试**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-repo/src/inventory.rs crates/erp-repo/src/products.rs crates/erp-repo/src/skus.rs crates/erp-repo/src/sku_batches.rs crates/erp-repo/src/atp_inventory.rs crates/erp-repo/src/product_packaging_specs.rs crates/erp-repo/src/inventory_reconciliation_center.rs apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx apps/erp-web/src/pages/products/ProductPackagingSpecsPage.tsx crates/erp-api/tests/inventory_atp_fefo_integration.rs
git commit -m "feat: add inventory atp fefo and packaging governance"
```

### Task 5: 计划补货与采购调拨优化

**Files:**
- Create: `F:/erp/crates/erp-api/src/planning.rs`
- Create: `F:/erp/crates/erp-repo/src/planning/demand_forecasts.rs`
- Create: `F:/erp/crates/erp-repo/src/planning/replenishment_rules.rs`
- Create: `F:/erp/crates/erp-repo/src/planning/purchase_plans.rs`
- Create: `F:/erp/crates/erp-repo/src/planning/transfer_suggestions.rs`
- Create: `F:/erp/crates/erp-repo/src/planning/promo_stocking.rs`
- Modify: `F:/erp/openapi/erp_api_v1.yaml`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/SalesForecastPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/ReplenishmentRulesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PurchasePlansPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/TransferSuggestionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PromoStockingPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PlanExecutionPage.tsx`
- Create: `F:/erp/crates/erp-api/tests/planning_replenishment_flow.rs`

- [ ] **Step 1: 写失败的预测、补货、采购计划、调拨建议测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test planning_replenishment_flow -- --nocapture`，确认失败**
- [ ] **Step 3: 把 `planning/*` 六个页面全部改为真实 API 驱动**
- [ ] **Step 4: 重跑后端测试与 planning 页面回归**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-api/src/planning.rs crates/erp-repo/src/planning openapi/erp_api_v1.yaml apps/erp-web/src/pages/planning crates/erp-api/tests/planning_replenishment_flow.rs
git commit -m "feat: add planning and replenishment domain"
```

### Task 6: 售后逆向履约、物流协同与订单级利润

**Files:**
- Modify: `F:/erp/crates/erp-repo/src/after_sales.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sale_cases.rs`
- Modify: `F:/erp/crates/erp-repo/src/logistics_exceptions.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_costing.rs`
- Create: `F:/erp/crates/erp-repo/src/profit_fact_orders.rs`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesExchangesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/logistics/LogisticsExceptionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ProfitPage.tsx`
- Create: `F:/erp/crates/erp-api/tests/after_sales_exchange_profit_flow.rs`

- [ ] **Step 1: 写失败的换货补发、逆向处置与利润归因测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test after_sales_exchange_profit_flow -- --nocapture`，确认失败**
- [ ] **Step 3: 落地换补发主流程、逆向处置矩阵与订单级利润事实表**
- [ ] **Step 4: 重跑测试**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-repo/src/after_sales.rs crates/erp-repo/src/after_sale_cases.rs crates/erp-repo/src/logistics_exceptions.rs crates/erp-repo/src/finance.rs crates/erp-repo/src/finance_costing.rs crates/erp-repo/src/profit_fact_orders.rs apps/erp-web/src/pages/afterSales/AfterSalesExchangesPage.tsx apps/erp-web/src/pages/logistics/LogisticsExceptionsPage.tsx apps/erp-web/src/pages/finance/ProfitPage.tsx crates/erp-api/tests/after_sales_exchange_profit_flow.rs
git commit -m "feat: add reverse fulfillment and order profit facts"
```

### Task 7: 工作流、自定义字段、审计报表后端化

**Files:**
- Create: `F:/erp/crates/erp-api/src/system_extensions.rs`
- Create: `F:/erp/crates/erp-repo/src/system_extensions/workflows.rs`
- Create: `F:/erp/crates/erp-repo/src/system_extensions/custom_fields.rs`
- Create: `F:/erp/crates/erp-repo/src/system_extensions/integration_center.rs`
- Create: `F:/erp/crates/erp-repo/src/system_extensions/audit_reports.rs`
- Modify: `F:/erp/openapi/erp_api_v1.yaml`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemWorkflowsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemCustomFieldsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemAuditReportsPage.tsx`
- Create: `F:/erp/infra/camunda/docker-compose.yml`
- Create: `F:/erp/apps/erp-web/e2e/system-extensions.spec.ts`

- [ ] **Step 1: 写失败的工作流、自定义字段、审计报表 API 测试**
- [ ] **Step 2: 运行失败测试，确认四个系统扩展页仍依赖本地存储**
- [ ] **Step 3: 后端化系统扩展能力，并接入 Camunda**
- [ ] **Step 4: 重跑 API 与前端 E2E**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-api/src/system_extensions.rs crates/erp-repo/src/system_extensions openapi/erp_api_v1.yaml apps/erp-web/src/pages/system/SystemWorkflowsPage.tsx apps/erp-web/src/pages/system/SystemCustomFieldsPage.tsx apps/erp-web/src/pages/system/SystemAuditReportsPage.tsx infra/camunda/docker-compose.yml apps/erp-web/e2e/system-extensions.spec.ts
git commit -m "feat: backendize system extension modules"
```

### Task 8: 专题统计、驾驶舱、SSO 与运维硬化

**Files:**
- Create: `F:/erp/crates/erp-analytics/Cargo.toml`
- Create: `F:/erp/crates/erp-analytics/src/lib.rs`
- Create: `F:/erp/crates/erp-analytics/src/clickhouse_sink.rs`
- Modify: `F:/erp/Cargo.toml`
- Modify: `F:/erp/apps/erp-web/src/api/stats.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SalesStatsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SalesTrendAnalysisPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/ShippingStatsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Create: `F:/erp/infra/clickhouse/docker-compose.yml`
- Create: `F:/erp/infra/keycloak/docker-compose.yml`
- Create: `F:/erp/docs/ops/SSO_RUNBOOK.md`
- Create: `F:/erp/crates/erp-api/tests/stats_clickhouse_sso_smoke.rs`

- [ ] **Step 1: 写失败的 stats、ClickHouse、SSO 烟囱测试**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test stats_clickhouse_sso_smoke -- --nocapture`，确认失败**
- [ ] **Step 3: 接入 ClickHouse 指标链路、Keycloak SSO 与业务 SLA 大盘**
- [ ] **Step 4: 重跑测试并执行 `pnpm --dir apps/erp-web build`**
- [ ] **Step 5: 提交**

```bash
git add crates/erp-analytics Cargo.toml apps/erp-web/src/api/stats.ts apps/erp-web/src/pages/stats apps/erp-web/src/pages/system/ObservabilityPage.tsx infra/clickhouse/docker-compose.yml infra/keycloak/docker-compose.yml docs/ops/SSO_RUNBOOK.md crates/erp-api/tests/stats_clickhouse_sso_smoke.rs
git commit -m "feat: add analytics stack and enterprise sso"
```
