# Industrial ERP 95%+ Completion Five-Track Codex Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 以工业级电商 ERP 首个可收费版本为目标，把当前项目从“页面很多但闭环不稳”的状态推进到 95%+ 可交付状态，并将执行主线收敛为 5 个可并行、可验证、可合并的子任务。

**Architecture:** 本计划采用“5 条业务闭环主线 + 真实进度吸收 + 严格文件边界 + 最终统一集成门禁”的推进方式。每条主线都直接绑定当前代码基础和既有 worktree 成果，不再按细碎模块平均拆分，而是按上线闭环与集成风险聚合，优先收口平台基线、渠道商品、OMS、履约补货、售后财务与最终门禁。

**Tech Stack:** React + TypeScript + Playwright + Rust + Axum + SQLx + PostgreSQL + OpenAPI + pnpm + cargo + Codex worktrees

---

## 0. 当前项目状态判断

当前项目已经具备“工业级电商 ERP 雏形”，但距离 95%+ 还有明显的结构性缺口，问题不在“没有页面”，而在“闭环收口、持久化证据、自动化验证、生产入口治理”还没有统一完成。

当前真实进展可归纳为：

1. `erp95-01` 已经开始处理生产菜单与路由暴露问题，说明生产入口治理不是空白。
2. `erp95-03` 已经在 `SystemSyncJobsPage.tsx` 和 `system-integrations.spec.ts` 上有实质推进，渠道运营工作台正在成形。
3. `erp95-04` 已经把 `ProductDiffPage.tsx` 往可用治理页推进，商品差异治理不应再按“占位页”估算。
4. `erp95-05` 已经在 `orders.rs` 与 `oms_orders.rs` 上补充 OMS 主链能力，订单域进入可收口阶段。
5. `erp95-06` 已经在波次、发货、出库确认页面上形成前端进展，但还没有完整验证证据。
6. `erp95-02` 出现了明显越界，说明旧的 10-track 方案过细，导致并行执行时边界污染和责任模糊。

结论：项目不是“低完成度重做”，而是“高覆盖度、低收口度”。因此本次重写主计划的目标，不是重新发明 5 个模块，而是把现有 10 轨成果重新编组成 5 条真正能推动上线的业务闭环主线。

## 1. 为什么从 10 Track 收敛到 5 Track

### 1.1 收敛原因

1. 旧 10-track 方案更像“能力地图均分”，不够贴近当前真实代码进度。
2. 过细拆分让共享边界增多，`ShopAccessPage.tsx`、菜单路由、集成门禁等文件天然容易冲突。
3. 当前环境验证成本高，后端和 E2E 门禁经常受机器资源影响，过多轨道只会提高集成摩擦。
4. 95%+ 的判定标准必须围绕“业务闭环是否可上线”，而不是“多少页面有内容”。
5. 已有工作树成果集中分布在 01、03、04、05、06，继续维持 10 轨只会让收口成本继续上升。

### 1.2 新旧轨道映射

| 新 Track | 吸收旧 Track | 主目标 |
| --- | --- | --- |
| Track-01 平台与生产入口基线 | 01 + 02 | 收口生产菜单、路由、租户/组织/权限最小基线 |
| Track-02 渠道接入与商品治理 | 03 + 04 | 收口渠道接入运营、同步任务、商品下载、映射治理 |
| Track-03 OMS 主链收口 | 05 | 收口审单、异常、拆合单、结算、审计落库 |
| Track-04 履约与补货执行闭环 | 06 + 07 | 收口波次、发货、库存、采购、补货计划与执行回写 |
| Track-05 售后财务与集成门禁 | 08 + 09 + 10 | 收口售后、财务、观测、门禁、证据包与最终 Go/No-Go |

### 1.3 新执行原则

1. 不再把“集成门禁”单独做成第 10 轨，而是并入 Track-05，由最终收口轨统一负责。
2. 新 5 轨必须吸收旧工作树中已经有价值的变更，禁止为了追求整齐而重做已经成形的成果。
3. 每条新轨道都必须有独占文件边界，禁止跨轨“顺手修一下”。
4. 任何业务功能的完成定义，仍然必须同时满足：有入口、有持久化、有自动化验证、有生产门禁证据。

## 2. 95%+ 完成度口径

### 2.1 总体达标条件

- [ ] `docs/spec/erp-launch-full-feature-matrix.md` 中所有 `P0` 项不再是 `缺失`
- [ ] 生产菜单不再暴露开发演示入口或“建设中”主入口
- [ ] 平台、渠道商品、OMS、履约补货、售后财务五条主线都至少有一条主链路可跑通
- [ ] 所有关键写操作都能提供持久化或审计证据
- [ ] `repo:verify`、后端 release gate、前端 build、E2E release gate 具备可执行路径
- [ ] 最终证据包可支持上线评审

### 2.2 当前不计入 95%+ 的伪完成

- 只有页面，没有真实写回或状态流转
- 接口成功，但没有数据库落库证据
- 有手工演示路径，但没有自动化回归
- 只有局部功能通过，但没有纳入生产门禁
- 单个 worktree 成功，但主仓集成不可验证

## 3. 执行总则

1. 本计划优先“收口”，不优先“扩面”。
2. 默认直接吸收现有 `erp95-*` worktree 的成果，不建议重新创建 5 套全新实现分支。
3. 如果确需新建 5 条执行分支，统一使用：
   - `codex/erp95-5t-01-platform-ia`
   - `codex/erp95-5t-02-channel-product`
   - `codex/erp95-5t-03-oms-main-chain`
   - `codex/erp95-5t-04-fulfillment-planning`
   - `codex/erp95-5t-05-aftersales-finance-gate`
4. 每次提交前必须执行边界校验；边界不干净的分支不得进入合并阶段。
5. 如果发现旧 worktree 中存在越界变更，先做“边界消毒”，再做功能推进。
6. 当前环境存在 `os error 1455`、链接内存不足、Playwright 资源受限等问题，因此本地验证失败不能直接等于逻辑失败，但必须在记录里明确标注“环境失败”还是“逻辑失败”。

## 4. 五轨主计划

### Task 1: 平台与生产入口基线

**吸收来源：**
- 旧 `erp95-01`
- 旧 `erp95-02`
- 主仓中已经落地的租户/权限相关未提交改动

**Files:**
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/moduleSearch.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/moduleSearch.test.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/sales/SalesEcommercePlaceholderPages.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/AgGridEnterpriseDemoPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/DataScopesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/UsersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/channels/ShopAccessPage.tsx`
- Modify: `F:/erp/crates/erp-repo/src/tenants.rs`
- Modify: `F:/erp/crates/erp-repo/src/data_scopes.rs`
- Modify: `F:/erp/crates/erp-repo/src/permissions.rs`
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Create: `F:/erp/migrations/0095_multi_merchant_org_baseline.sql`
- Test: `F:/erp/crates/erp-api/tests/api_rbac_tenant_isolation.rs`
- Test: `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/frontend-route-observability.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/channels-shops-list-visibility.spec.ts`

- [ ] **Step 1: 清理边界污染并盘点已有成果**
  Run: `git -C F:/erp/.worktrees/erp95-02 diff --stat`
  Expected: 明确哪些文件属于平台基线，哪些文件必须剥离出 Track-01

- [ ] **Step 2: 先补失败断言，锁定生产入口与租户隔离口径**
  Run: `cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture`
  Expected: 若失败，失败原因能明确指向租户/组织/店铺隔离缺口

- [ ] **Step 3: 完成生产菜单与路由治理**
  Outcome: 生产菜单不再把开发演示或建设中页面作为主入口暴露；菜单、路由、模块搜索保持一致

- [ ] **Step 4: 完成最小多商户/组织/数据权限基线**
  Outcome: 形成最小可发版的 merchant/org/shop 级权限与展示能力，不扩展到复杂法人/结算模型

- [ ] **Step 5: 验证前端入口与权限隔离**
  Run: `pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/frontend-route-observability.spec.ts e2e/channels-shops-list-visibility.spec.ts --project=chromium`
  Expected: 关键入口与可见性回归通过，或输出明确的环境阻塞证据

- [ ] **Step 6: 记录平台基线假设与剩余边界**
  Outcome: 文档明确“本次只做最小可上线平台基线，不扩展到多法人/总账级主数据”

**Task DoD:**
- 生产入口不再暴露明显演示页面或占位页
- 租户/组织/店铺最小隔离基线可验证
- `api_rbac_tenant_isolation` 与导航相关测试具备通过路径
- 无越界修改渠道、商品、OMS、WMS、财务业务文件

**Verification Commands:**

```bash
cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture
pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts
pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/frontend-route-observability.spec.ts e2e/channels-shops-list-visibility.spec.ts --project=chromium
```

### Task 2: 渠道接入与商品治理

**吸收来源：**
- 旧 `erp95-03`
- 旧 `erp95-04`

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/SystemSyncJobsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/systemIntegrations.ts`
- Modify: `F:/erp/apps/erp-web/src/api/syncJobs.ts`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductChangeControlPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductChannelAvailabilityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/products/ProductDiffPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/api/products.ts`
- Modify: `F:/erp/apps/erp-web/src/api/shopProducts.ts`
- Modify: `F:/erp/crates/erp-repo/src/sync_jobs.rs`
- Modify: `F:/erp/crates/erp-repo/src/sync_job_runs.rs`
- Modify: `F:/erp/crates/erp-repo/src/platform_tokens.rs`
- Modify: `F:/erp/crates/erp-repo/src/products.rs`
- Modify: `F:/erp/crates/erp-repo/src/shop_products.rs`
- Modify: `F:/erp/crates/erp-repo/src/channel_sku_mappings.rs`
- Modify: `F:/erp/crates/erp-repo/src/shop_product_tasks.rs`
- Test: `F:/erp/crates/erp-api/tests/platform_oauth_webhook_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/system_integrations_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs`
- Test: `F:/erp/apps/erp-web/e2e/system-integrations.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/shop-product-match-governance.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/product-change-control.spec.ts`

- [ ] **Step 1: 固化渠道接入和商品治理的失败测试口径**
  Run: `cargo test -p erp-api --test platform_oauth_webhook_integration --test system_integrations_integration --test platform_sync_and_mapping_flow_integration -- --nocapture`
  Expected: 失败点能清晰归因到授权状态、同步运行态、映射或差异治理缺口

- [ ] **Step 2: 完成渠道接入运营面板**
  Outcome: 前端可展示授权状态、最近运行、失败原因、人工重试/补偿入口

- [ ] **Step 3: 完成商品下载、映射、差异分析与变更控制闭环**
  Outcome: `ProductDiffPage` 不再是占位页，下载任务、映射结果、冲突原因和回写结果可追踪

- [ ] **Step 4: 对齐接入状态与商品治理数据模型**
  Outcome: 渠道 token、同步作业、店铺商品、主商品映射使用同一套可解释状态口径

- [ ] **Step 5: 跑通后端集成测试与前端 E2E**
  Run: `pnpm --filter erp-web exec playwright test e2e/system-integrations.spec.ts e2e/shop-product-match-governance.spec.ts e2e/product-change-control.spec.ts --project=chromium`
  Expected: 渠道运营与商品治理主链路可回归

- [ ] **Step 6: 输出映射治理剩余风险**
  Outcome: 明确本轮只收口“首发可用治理”，不扩展复杂自动匹配策略平台

**Task DoD:**
- 渠道授权、同步、失败补偿、商品下载、映射治理形成一条运营闭环
- `ProductDiffPage`、`SystemSyncJobsPage` 成为真实工作台
- 后端集成测试与商品治理 E2E 具备通过路径
- 不修改租户权限、OMS、WMS、财务相关越界文件

**Verification Commands:**

```bash
cargo test -p erp-api --test platform_oauth_webhook_integration --test system_integrations_integration --test platform_sync_and_mapping_flow_integration -- --nocapture
pnpm --filter erp-web exec playwright test e2e/system-integrations.spec.ts e2e/shop-product-match-governance.spec.ts e2e/product-change-control.spec.ts --project=chromium
```

### Task 3: OMS 主链收口

**吸收来源：**
- 旧 `erp95-05`

**Files:**
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

- [ ] **Step 1: 固化 OMS 主链的失败测试**
  Run: `cargo test -p erp-api --test oms_orders --test order_exception_sla_integration --test order_match_ship_wave_persistence_guard -- --nocapture`
  Expected: 失败点应能准确定位到审单、异常、状态链、履约衔接或持久化缺口

- [ ] **Step 2: 收口审单、异常、拆合单、复核、结算动作**
  Outcome: 关键动作从前端到后端都能形成统一状态链，且写入审计日志

- [ ] **Step 3: 补齐操作日志和状态历史查询**
  Outcome: 任一关键写操作都能追踪到是谁、何时、因为什么做了变更

- [ ] **Step 4: 对齐 OMS 与履约的接口契约**
  Outcome: 不在 Track-03 直接修改 WMS 业务文件，但要保证 OMS 输出状态可被履约链消费

- [ ] **Step 5: 运行主链 E2E**
  Run: `pnpm --filter erp-web exec playwright test e2e/order-processing-flow.spec.ts e2e/order-exceptions.spec.ts --project=chromium`
  Expected: 审单与异常处理主链具备回归能力

- [ ] **Step 6: 输出 OMS 收口证据表**
  Outcome: 明确哪些动作有数据库证据，哪些仅完成页面收口但尚待联调

**Task DoD:**
- 审单、异常、拆合单、复核、结算形成可解释状态链
- 审计日志和状态历史可查询
- OMS 后端测试和前端主链 E2E 具备通过路径
- 不越界修改 WMS、采购、售后、财务文件

**Verification Commands:**

```bash
cargo test -p erp-api --test oms_orders --test order_exception_sla_integration --test order_match_ship_wave_persistence_guard -- --nocapture
pnpm --filter erp-web exec playwright test e2e/order-processing-flow.spec.ts e2e/order-exceptions.spec.ts --project=chromium
```

### Task 4: 履约与补货执行闭环

**吸收来源：**
- 旧 `erp95-06`
- 旧 `erp95-07`

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/ProcurementRequestsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/ProcurementPlansPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/PurchaseOrdersPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/PurchaseReceiptsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/procurement/PurchaseSuggestionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/ReplenishmentRulesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PurchasePlansPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/planning/PlanExecutionPage.tsx`
- Modify: `F:/erp/crates/erp-repo/src/wms.rs`
- Modify: `F:/erp/crates/erp-repo/src/shipments.rs`
- Modify: `F:/erp/crates/erp-repo/src/inventory.rs`
- Modify: `F:/erp/crates/erp-repo/src/atp_inventory.rs`
- Modify: `F:/erp/crates/erp-api/src/planning.rs`
- Modify: `F:/erp/crates/erp-repo/src/procurement.rs`
- Modify: `F:/erp/crates/erp-repo/src/planning.rs`
- Modify: `F:/erp/crates/erp-repo/src/suppliers.rs`
- Test: `F:/erp/crates/erp-api/tests/shipment_confirm.rs`
- Test: `F:/erp/crates/erp-api/tests/wave_ship_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/inventory_atp_fefo_integration.rs`
- Test: `F:/erp/crates/erp-api/tests/wms_labor_slotting_flow.rs`
- Test: `F:/erp/crates/erp-api/tests/planning_replenishment_flow.rs`
- Test: `F:/erp/crates/erp-repo/tests/procurement_flow.rs`
- Test: `F:/erp/apps/erp-web/e2e/wms-wave-main-chain.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/procurement-pages.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/planning-flow.spec.ts`

- [ ] **Step 1: 拆开履约链与补货链的失败测试**
  Run: `cargo test -p erp-api --test shipment_confirm --test wave_ship_integration --test inventory_atp_fefo_integration --test wms_labor_slotting_flow --test planning_replenishment_flow -- --nocapture`
  Expected: 能区分是履约侧问题、库存一致性问题，还是补货计划执行问题

- [ ] **Step 2: 收口波次、出库、发货确认和 ATP 一致性**
  Outcome: 履约主链至少能支持部分失败、重试、回放的状态证据

- [ ] **Step 3: 收口采购建议、采购单、收货、补货计划、执行反馈**
  Outcome: 采购和计划不再只是展示页，而是有真正的执行回写

- [ ] **Step 4: 对齐库存、补货与履约的共享口径**
  Outcome: 不把采购计划做成独立孤岛，而是与库存和出库形成一条执行链

- [ ] **Step 5: 运行履约与补货相关 E2E**
  Run: `pnpm --filter erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts e2e/procurement-pages.spec.ts e2e/planning-flow.spec.ts --project=chromium`
  Expected: 履约与补货都具备页面级回归路径

- [ ] **Step 6: 输出“履约失败补偿与补货边界”说明**
  Outcome: 明确本轮不扩展 PDA、自动化设备、复杂供应商绩效平台

**Task DoD:**
- 波次、出库、发货确认、库存一致性形成履约闭环
- 采购建议、采购计划、采购单、收货、执行反馈形成补货闭环
- WMS 和补货测试具备通过路径
- 不越界修改 OMS、售后、财务、门禁文件

**Verification Commands:**

```bash
cargo test -p erp-api --test shipment_confirm --test wave_ship_integration --test inventory_atp_fefo_integration --test wms_labor_slotting_flow --test planning_replenishment_flow -- --nocapture
cargo test -p erp-repo --test procurement_flow -- --nocapture
pnpm --filter erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts e2e/procurement-pages.spec.ts e2e/planning-flow.spec.ts --project=chromium
```

### Task 5: 售后财务与集成门禁

**吸收来源：**
- 旧 `erp95-08`
- 旧 `erp95-09`
- 旧 `erp95-10`

**Files:**
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesExchangesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesInspectionsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/afterSales/RefundDocsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/customers/CustomerServiceTicketsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ArManagementPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ApManagementPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ReconcilePage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ExpenseAllocationPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/ProfitPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/finance/InvoicesPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/ObservabilityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/system/frontendRouteObservability.ts`
- Modify: `F:/erp/crates/erp-repo/src/after_sales.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sales_sla_rules.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sales_quality_rules.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sales_intercepts.rs`
- Modify: `F:/erp/crates/erp-repo/src/after_sale_cases.rs`
- Modify: `F:/erp/crates/erp-api/src/finance_management.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_ar.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_ap_plans.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_expenses.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_costing.rs`
- Modify: `F:/erp/crates/erp-repo/src/finance_tax.rs`
- Modify: `F:/erp/package.json`
- Modify: `F:/erp/.github/workflows/ci.yml`
- Modify: `F:/erp/scripts/release_gate_backend.mjs`
- Modify: `F:/erp/scripts/verify_openapi_gate_v2.mjs`
- Modify: `F:/erp/scripts/verify_repo_baseline.mjs`
- Modify: `F:/erp/docs/ops/RELEASE_GATE_CHECKLIST_V2.md`
- Modify: `F:/erp/docs/ops/GO_NO_GO_CHECKLIST_V2.md`
- Modify: `F:/erp/docs/ops/TASK_COMPLETION_MATRIX_V2.md`
- Modify: `F:/erp/docs/ops/progress_check.md`
- Modify: `F:/erp/docs/spec/erp-launch-full-feature-matrix.md`
- Modify: `F:/erp/infra/observability/prometheus/alerts.yml`
- Test: `F:/erp/crates/erp-api/tests/after_sales.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_state_machine_guard.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_sla_escalation.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_exchange_profit_flow.rs`
- Test: `F:/erp/crates/erp-api/tests/finance_reconcile.rs`
- Test: `F:/erp/crates/erp-api/tests/after_sales_finance_reconcile.rs`
- Test: `F:/erp/crates/erp-repo/tests/finance_profit.rs`
- Test: `F:/erp/apps/erp-web/e2e/after-sales-workflow.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/customer-service-tickets.spec.ts`
- Test: `F:/erp/apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts`

- [ ] **Step 1: 固化售后和财务闭环的失败测试**
  Run: `cargo test -p erp-api --test after_sales --test after_sales_state_machine_guard --test after_sales_sla_escalation --test after_sales_exchange_profit_flow --test finance_reconcile --test after_sales_finance_reconcile -- --nocapture`
  Expected: 能区分售后状态机问题、SLA 问题、财务回写问题和利润口径问题

- [ ] **Step 2: 收口售后主链**
  Outcome: 客服工单、售后单、逆向收货、质检、补发、退款单据形成一条业务链

- [ ] **Step 3: 收口首发财务闭环**
  Outcome: AR/AP/对账/费用分摊/利润/开票形成最小可经营分析闭环，不扩展总账平台

- [ ] **Step 4: 把售后财务联动纳入观测与门禁**
  Outcome: 关键链路有可观测字段、告警项、门禁脚本和矩阵更新

- [ ] **Step 5: 跑通售后、财务、全局门禁**
  Run: `pnpm run repo:verify`
  Expected: 至少形成一条完整的门禁执行路径，并沉淀失败证据或通过证据

- [ ] **Step 6: 生成最终证据包与 Go/No-Go 结论**
  Outcome: 更新能力矩阵、门禁清单、进度检查表，给出是否达到 95%+ 的审慎结论

**Task DoD:**
- 售后、财务、门禁、观测与证据包形成最终收口轨
- 首发经营会计闭环可解释，不强行扩展到总账
- 所有门禁脚本、矩阵、观测页面与告警配置同步更新
- 最终 Go/No-Go 结论有依据

**Verification Commands:**

```bash
cargo test -p erp-api --test after_sales --test after_sales_state_machine_guard --test after_sales_sla_escalation --test after_sales_exchange_profit_flow --test finance_reconcile --test after_sales_finance_reconcile -- --nocapture
cargo test -p erp-repo --test finance_profit -- --nocapture
pnpm --filter erp-web exec playwright test e2e/after-sales-workflow.spec.ts e2e/after-sales-return-resend-main-chain.spec.ts e2e/customer-service-tickets.spec.ts e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium
pnpm run repo:verify
pnpm run test:backend:release-gate
pnpm --dir apps/erp-web run generate:api
pnpm --dir apps/erp-web build
pnpm run test:e2e:release-gate
```

## 5. 文件边界与禁改规则

### 5.1 共享禁改原则

1. 非 Track-01 禁改：
   - `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
   - `F:/erp/apps/erp-web/src/app/router.tsx`
   - `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
   - `F:/erp/apps/erp-web/src/app/layout/moduleSearch.ts`
2. 非 Track-05 禁改：
   - `F:/erp/package.json`
   - `F:/erp/.github/workflows/ci.yml`
   - `F:/erp/scripts/release_gate_backend.mjs`
   - `F:/erp/scripts/verify_openapi_gate_v2.mjs`
   - `F:/erp/scripts/verify_repo_baseline.mjs`
   - `F:/erp/docs/ops/**`
   - `F:/erp/infra/observability/prometheus/alerts.yml`
3. `F:/erp/apps/erp-web/src/pages/channels/ShopAccessPage.tsx` 归 Track-01 所有，Track-02 只能消费其接口结果，不得争抢此文件。
4. OMS 与履约的边界以“订单状态输出”和“履约状态消费”为准，Track-03 不改 WMS 文件，Track-04 不改 OMS 文件。
5. 售后与财务的联动由 Track-05 统一负责，Track-03 和 Track-04 不在本轮跨域写财务文件。

### 5.2 越界处理规则

- [ ] 一旦发现某条轨道修改了白名单之外的文件，先做边界修复，再继续开发
- [ ] 对于旧 worktree 中已存在的越界改动，优先剥离到正确轨道或回退到主线前状态
- [ ] 禁止用“为了测通”作为越界理由

## 6. 合并顺序

1. Track-01 平台与生产入口基线
2. Track-02 渠道接入与商品治理
3. Track-03 OMS 主链收口
4. Track-04 履约与补货执行闭环
5. Track-05 售后财务与集成门禁

说明：

1. Track-01 必须最先收口，因为它决定生产入口和数据隔离基线。
2. Track-02 必须早于 OMS/WMS 大规模联调，因为渠道与商品治理决定后续订单和履约数据质量。
3. Track-03 与 Track-04 可以并行推进，但合并时 OMS 应先于履约，以保证状态契约稳定。
4. Track-05 永远最后合并，因为它负责最终门禁、矩阵更新和 Go/No-Go 结论。

## 7. 统一 DoD

### 7.1 业务 DoD

- [ ] 五条主线都各自形成至少一条端到端业务链
- [ ] 生产菜单只暴露首发能力，不暴露开发样例页
- [ ] 关键写操作均能追溯到数据库证据或审计日志

### 7.2 测试 DoD

- [ ] 每条主线至少有一组后端集成测试
- [ ] 每条主线至少有一组前端 E2E 或页面级行为测试
- [ ] 最终门禁命令有实际执行记录

### 7.3 文档 DoD

- [ ] `docs/spec/erp-launch-full-feature-matrix.md` 更新为最新状态
- [ ] `docs/ops/progress_check.md`、门禁清单和任务矩阵同步更新
- [ ] 最终证据包能支撑是否达到 95%+ 的结论

## 8. 风险、止损与回滚

1. 如果 Track-01 发现多商户/多组织模型改动过大，只保留最小 merchant/org/shop 权限基线，不扩展复杂法人结算。
2. 如果 Track-02 的自动映射策略迟迟不稳，优先保住手工治理、冲突原因、任务追踪和变更控制。
3. 如果 Track-03 和 Track-04 联调受阻，优先确保状态契约清晰和持久化证据存在，不强行一次性跑通所有页面。
4. 如果 Track-05 的财务深度不足，先守住对账、利润、售后财务回写和开票最小闭环，不扩展总账。
5. 如果本地环境持续触发资源型失败，必须把结果迁移到 CI 或更高资源机器上重跑，不允许把“环境失败”包装成“功能完成”。

## 9. 最终交付物

- [ ] 1 份新的 5-track 主计划文档
- [ ] 5 条收敛后的开发主线及其文件边界
- [ ] 更新后的能力矩阵与进度结论
- [ ] 最终门禁执行记录
- [ ] Go/No-Go 证据包
- [ ] 对“是否达到工业级 95%+ 完成度”的审慎评估
