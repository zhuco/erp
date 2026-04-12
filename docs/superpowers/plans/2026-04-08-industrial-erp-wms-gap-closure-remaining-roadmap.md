# 工业级电商 ERP/WMS 剩余缺口收口实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不推翻当前仓库主栈的前提下，把现有系统从“功能已覆盖、工业深度不足”推进到“可支撑多平台电商、复杂仓配、计划补货、经营分析、企业治理”的工业级 ERP/WMS。

**Architecture:** 继续保留 `Rust + Postgres + NATS + React + OpenAPI` 作为交易主链，按“平台集成生产化 -> OMS/WMS 工业化 -> 计划与利润经营 -> 数据与身份治理硬化”推进。流程编排统一接入 `Camunda 8`，专题分析统一接入 `ClickHouse`，企业级身份统一接入 `Keycloak`。

**Tech Stack:** Rust、SQLx、Postgres、NATS、OpenAPI、React、Vite、Ant Design、Playwright、Camunda 8、ClickHouse、Keycloak

---

## 当前已完成基线

- [x] SaaS 底座：租户、RBAC、数据权限、审计日志、特性开关、基础可观测能力
- [x] OMS/WMS 主链：订单、采购、入库、出库、波次、拣货、打包、盘点、调拨等基础流程
- [x] 系统扩展后端化第一阶段：
  - [x] `SystemWorkflowsPage.tsx`
  - [x] `SystemCustomFieldsPage.tsx`
  - [x] `SystemIntegrationsPage.tsx`
  - [x] `SystemAuditReportsPage.tsx`
- [x] 四个系统扩展页面均已改为真实后端 `settings` catalog 持久化，并具备权限校验与后端集成测试

## 剩余缺口总览

### P0：必须先补齐

- 平台接入生产化闭环：真实 OAuth、Webhook 验签、字段映射、回调幂等、同步 SLA、失败重试与告警
- OMS 异常中台：异常 SLA、拆单合单、结算对账、自动派单与超时催办
- WMS 现场执行：PDA/RF、Labor、Slotting、补货、月台/场院、设备协同
- 计划补货域：需求预测、补货规则、采购计划、调拨建议、促销备货、计划执行看板
- 企业治理硬化：Camunda 工作流编排、Keycloak SSO、ClickHouse 指标中台、系统级可观测与业务 SLA

### P1：工业深度增强

- ATP、FEFO、多包装层级、库存对账中心
- 逆向履约、换补发、物流异常协同、订单级利润
- 全链路 E2E、OpenAPI 补齐、性能与容量测试、运维 Runbook

### P2：体验与经营升级

- 经营驾驶舱、利润钻取、库存健康度、渠道/仓/商品多维专题
- 更细颗粒度的自动化编排、策略仿真、智能预警与推荐

## 分类缺口与落地任务

### Task 1：平台接入生产化与集成中心闭环

**目标：** 把当前“可配置的集成中心”升级为“真实可联调、可回调、可审计、可追 SLA”的平台接入中台。

**Files:**
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Modify: `F:/erp/crates/erp-channel/src/main.rs`
- Modify: `F:/erp/crates/erp-worker/src/main.rs`
- Modify: `F:/erp/crates/erp-repo/src/platform_tokens.rs`
- Modify: `F:/erp/crates/erp-repo/src/settings.rs`
- Modify: `F:/erp/openapi/erp_api_v1.yaml`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/systemIntegrations.ts`
- Create: `F:/erp/crates/erp-api/tests/platform_oauth_webhook_integration.rs`
- Create: `F:/erp/apps/erp-web/e2e/system-integrations.spec.ts`

- [ ] **Step 1: 先写失败测试，覆盖 OAuth 发起、回调验签、Webhook 幂等、同步任务 SLA**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture`，确认红灯**
- [ ] **Step 3: 落地真实平台凭证管理、回调状态机、Webhook 签名校验、失败重试与告警落库**
- [ ] **Step 4: 把集成中心页面从“通用配置表”升级为“授权状态 + 健康状态 + 最近同步 + 异常告警”工作台**
- [ ] **Step 5: 跑通 `pnpm --dir apps/erp-web test:e2e -- e2e/system-integrations.spec.ts`**

### Task 2：OMS 工业化与订单异常 SLA 中台

**目标：** 形成电商订单异常池、审核池、结算对账池三套工业作业台。

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

- [ ] **Step 1: 先写失败测试，覆盖异常 SLA、拆单合单、结算对账、升级与超时**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test order_exception_sla_integration -- --nocapture`，确认红灯**
- [ ] **Step 3: 建立异常池、动作台账、状态轨迹、超时规则与升级动作**
- [ ] **Step 4: 改造 OMS 前端页面，形成审核池、异常池、对账池的可操作工作台**
- [ ] **Step 5: 跑后端测试与 OMS 页面的 E2E**

### Task 3：WMS 现场执行、Labor、Slotting、补货与月台

**目标：** 从后台列表式 WMS 升级为现场执行型 WMS。

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

- [ ] **Step 1: 先写失败测试，覆盖补货、Slotting、PDA、交接台与月台**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture`，确认红灯**
- [ ] **Step 3: 落地现场执行层，包括人员任务、设备协同、库位优化与补货策略**
- [ ] **Step 4: 改造波次、拣货、打包、发运页面，并补 PDA 端页面**
- [ ] **Step 5: 跑 `e2e/wms-wave-main-chain.spec.ts`、`e2e/wms-handover.spec.ts` 与新增 PDA 回归**

### Task 4：库存承诺、效期与主数据治理

**目标：** 形成库存承诺能力，而不是只做库存台账。

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

- [ ] **Step 1: 先写失败测试，覆盖 ATP、FEFO、包装层级、库存对账**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture`，确认红灯**
- [ ] **Step 3: 落地 ATP 可承诺库存、效期分配、包装规格与对账中心**
- [ ] **Step 4: 让库存与商品页面支持工业化库存治理视图**
- [ ] **Step 5: 跑集成测试与前端类型检查**

### Task 5：计划补货、采购计划与调拨建议

**目标：** 建立从预测到执行的计划闭环。

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

- [ ] **Step 1: 先写失败测试，覆盖预测、补货、采购计划、调拨建议、促销备货**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test planning_replenishment_flow -- --nocapture`，确认红灯**
- [ ] **Step 3: 新建 planning 域，形成“预测 -> 目标库存 -> 建议 -> 执行 -> 回流”的闭环**
- [ ] **Step 4: 把 `planning/*` 六个页面全部改为真实 API 驱动**
- [ ] **Step 5: 跑 planning 回归与前端类型检查**

### Task 6：售后逆向履约、物流协同与利润经营

**目标：** 形成售后、物流、财务联动的利润经营链路。

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

- [ ] **Step 1: 先写失败测试，覆盖换补发、逆向处置、物流索赔与订单级利润**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test after_sales_exchange_profit_flow -- --nocapture`，确认红灯**
- [ ] **Step 3: 打通售后、物流、财务三域，产出订单级利润事实表**
- [ ] **Step 4: 改造售后、物流异常、利润页面**
- [ ] **Step 5: 跑后端测试与前端类型检查**

### Task 7：流程编排、企业治理与系统扩展第二阶段

**目标：** 把当前已经后端化的系统扩展模块继续升级为可编排、可审批、可人机协同的治理中台。

**Files:**
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Modify: `F:/erp/openapi/erp_api_v1.yaml`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemWorkflowsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemCustomFieldsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemAuditReportsPage.tsx`
- Create: `F:/erp/infra/camunda/docker-compose.yml`
- Create: `F:/erp/apps/erp-web/e2e/system-governance.spec.ts`

- [ ] **Step 1: 先写失败测试，覆盖 BPMN 审批、DMN 规则、人任务分配与配置生效**
- [ ] **Step 2: 运行新增后端测试，确认红灯**
- [ ] **Step 3: 接入 Camunda 8，承接审批、人任务、规则决策、跨系统编排**
- [ ] **Step 4: 为四个系统扩展页面补工作流状态、发布状态、审批链与生效记录**
- [ ] **Step 5: 跑 API、E2E 与类型检查**

### Task 8：统计分析、控制塔、SSO 与可观测硬化

**目标：** 形成工业级系统最关键的“看得见、管得住、登得上、告得准”能力。

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

- [ ] **Step 1: 先写失败测试，覆盖 ClickHouse 指标链路、Keycloak 登录、业务 SLA 告警**
- [ ] **Step 2: 运行 `cargo test -p erp-api --test stats_clickhouse_sso_smoke -- --nocapture`，确认红灯**
- [ ] **Step 3: 接入 ClickHouse 作为专题分析层，接入 Keycloak 作为统一身份层**
- [ ] **Step 4: 把 `stats/*` 从 placeholder 改为真实专题页面，并补系统可观测总览**
- [ ] **Step 5: 运行 `pnpm --dir apps/erp-web build`、类型检查与烟囱测试**

### Task 9：工业级非功能能力补齐

**目标：** 把“能用”升级为“可持续运行、可扩容、可追责、可恢复”。

**Files:**
- Modify: `F:/erp/openapi/erp_api_v1.yaml`
- Modify: `F:/erp/apps/erp-web/e2e/*`
- Modify: `F:/erp/crates/erp-api/tests/*`
- Create: `F:/erp/docs/ops/DR_RUNBOOK.md`
- Create: `F:/erp/docs/ops/PERF_BASELINE.md`
- Create: `F:/erp/docs/ops/ALERT_RULES.md`

- [ ] **Step 1: 统一补 OpenAPI 契约、错误码、鉴权说明与回调契约**
- [ ] **Step 2: 补关键业务链路 E2E：平台拉单、OMS 审核、WMS 发运、售后逆向、计划执行**
- [ ] **Step 3: 建立性能基线、容量基线、恢复演练文档与告警规则**
- [ ] **Step 4: 建立任务幂等、补偿、重放、审计可追溯的验收清单**
- [ ] **Step 5: 形成上线门禁：测试、性能、回滚、告警、审计、Runbook 全部齐备**

## 推荐执行顺序

1. `Task 1 平台接入生产化`
2. `Task 2 OMS 异常中台`
3. `Task 3 WMS 现场执行`
4. `Task 5 计划补货域`
5. `Task 4 库存承诺与主数据治理`
6. `Task 6 售后逆向与利润经营`
7. `Task 7 Camunda 企业治理`
8. `Task 8 统计分析与 SSO`
9. `Task 9 非功能硬化`

## 技术选型原则

- 工作流与审批编排：选 `Camunda 8`
  - 原因：同时支持 BPMN、DMN、人任务、表单、连接器和跨系统编排，最适合“业务可视化 + 开发可控”的工业 ERP/WMS
- 专题分析与控制塔：选 `ClickHouse`
  - 原因：比继续堆 Postgres 聚合更适合高并发、实时专题分析、明细钻取与长时间序列
- 企业级统一身份：选 `Keycloak`
  - 原因：开箱支持 OIDC/OAuth2/SAML、身份代理、LDAP/AD 联邦与集中管理，避免自研认证泥潭
- WMS 能力标杆：对齐 `Manhattan Active WM / Oracle WMS / SAP EWM`
  - 原因：这三类产品覆盖了工业级仓配对“Labor、Slotting、Automation、Yard、Omnichannel、Visibility”的核心能力线
- 计划补货标杆：对齐 `SAP IBP`
  - 原因：需求预测、库存优化、供需平衡、需求感知、协同计划是成熟能力线，适合作为内部实现边界

## 执行约束

- 当前目录 `F:/erp` 不是 Git 仓库，计划中的提交步骤只能在真实 Git 工作树中执行
- 每个任务都必须遵守 TDD：先写失败测试，再实现，再回归验证
- 每个任务都必须产出真实 API、真实持久化、真实权限校验，禁止继续新增 `localStorage` 演示态
- 每个阶段结束都必须补前端类型检查与后端集成回归
