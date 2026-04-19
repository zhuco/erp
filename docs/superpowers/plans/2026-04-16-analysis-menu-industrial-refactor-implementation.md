# Analysis Menu Industrial Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the ERP analysis menu into a problem-oriented “insights” domain, migrate existing stats/report/export pages into the new IA, and leave behind a parallel-safe execution plan for multiple Codex workers.

**Architecture:** Use an integrator-owned shared file lock for menu, route, title, and API registry files; let domain workers implement isolated insight page packs under the new `/insights/*` canonical routes. Phase 1 reuses current `stats`/`reports`/`exports` contracts, while Phase 2 expands theme-specific contracts without breaking the navigation shell.

**Tech Stack:** React, TypeScript, React Router, Ant Design, React Query, Playwright, Vitest

---

### Task 1: Integrator-Owned IA Shell And Shared File Lock

**Files:**
- Create: `F:/erp/apps/erp-web/src/features/insights/insightFeatureCatalog.ts`
- Create: `F:/erp/apps/erp-web/src/features/insights/insightDimensionCatalog.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/layout/headerMenuModel.ts`
- Modify: `F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
- Test: `F:/erp/apps/erp-web/src/app/layout/headerMenuModel.test.ts`

- [ ] **Step 1: Create the feature catalog and dimension catalog**

Create a catalog that defines, at minimum:

- `key`
- `title`
- `group`
- `canonicalPath`
- `legacyPaths`
- `ownerDomain`
- `viewKind`
- `defaultDimensions`
- `availabilityPhase`

Create a dimension catalog that normalizes shared filter semantics such as `platform`, `shop`, `warehouse`, `category`, `agent`, and `age_bucket`.

- [ ] **Step 2: Wire the header menu to the new insights IA**

Update the top-level menu item from `分析` to `洞察`, and make its groups match the new design:

- 总览快报
- 收入与订单质量
- 履约与仓配效率
- 商品结构与生命周期
- 库存健康与调拨
- 售后与补偿治理
- 报表与导出工坊

Do not expose every future page immediately. Only expose Phase 1 entries plus clearly marked coming-soon placeholders when the product decision is to surface them.

- [ ] **Step 3: Add route aliases and route titles**

Map existing legacy routes to canonical insight routes:

- `/stats/sales` -> `/insights/revenue/overview`
- `/stats/sales-daily` -> `/insights/revenue/daily-brief`
- `/stats/sales-trends` -> `/insights/revenue/trend-radar`
- `/stats/shipping` -> `/insights/fulfillment/lead-time`
- `/stats/slow-moving` -> `/insights/catalog/slow-sku`
- `/stats/best-sellers` -> `/insights/catalog/hot-sku`
- `/stats/sales-cycle` -> `/insights/fulfillment/order-cycle`
- `/reports` -> `/insights/workbench/report-center`
- `/exports` -> `/insights/workbench/export-queue`

Update title resolution so all new canonical routes render meaningful page titles.

- [ ] **Step 4: Run unit tests for menu model changes**

Run: `pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.ts`

Expected: PASS

- [ ] **Step 5: Record the shared file lock**

Before dispatching any worker, record in the task pack that the following files are integrator-owned and cannot be concurrently edited:

- `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- `F:/erp/apps/erp-web/src/app/router.tsx`
- `F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts`
- `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
- `F:/erp/apps/erp-web/src/api/stats.ts`
- `F:/erp/apps/erp-web/src/api/reports.ts`

### Task 2: Worker-A Revenue Insight Pack

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/insights/revenue/RevenueOverviewPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/revenue/DailyBriefPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/revenue/TrendRadarPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/revenue/OrderMixPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/revenue/DiscountInsightPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SalesStatsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SalesDailyReportPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SalesTrendAnalysisPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/StatsPlaceholder.tsx`
- Test: `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`

- [ ] **Step 1: Wrap existing stats pages in new canonical insight pages**

Phase 1 should avoid deep reimplementation. Build new page components that either:

- reuse current `StatsPlaceholder` wiring with new titles and route identities, or
- adapt current `SalesStatsPage`, `SalesDailyReportPage`, and `SalesTrendAnalysisPage` into new domain page wrappers

for:

- 交易总览
- 营运日报
- 增长雷达

- [ ] **Step 2: Extend the shared stats page skeleton for domain metadata**

Add support for catalog-driven page metadata such as:

- recommended dimensions
- domain badge
- capability description
- availability phase

without breaking old stats pages during the migration window.

- [ ] **Step 3: Prepare low-risk placeholders for new-but-not-yet-backed pages**

Create read-only placeholder surfaces for:

- 订单结构
- 让利诊断

Each placeholder must:

- explain the business question
- show planned dimensions
- show expected metrics
- clearly indicate “Phase 2 contract pending”

- [ ] **Step 4: Run focused revenue tests**

Run: `pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts`

Expected: PASS or only unrelated pre-existing failures

- [ ] **Step 5: Deliver worker summary**

Return:

- commit hash
- changed files
- tests run
- remaining contract gaps for `订单结构` and `让利诊断`

### Task 3: Worker-B Fulfillment Insight Pack

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/insights/fulfillment/LeadTimePage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/fulfillment/OperationLedgerPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/fulfillment/PackagingConsumptionPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/fulfillment/StocktakeVariancePage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/fulfillment/AccuracyControlPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/fulfillment/WarehouseProductivityPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/ShippingStatsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SalesCyclePage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`

- [ ] **Step 1: Migrate the existing shipping and cycle pages into the new fulfillment domain**

Build canonical route pages for:

- 履约时效
- 订单履约周期

These should reuse existing contracts first.

- [ ] **Step 2: Add phase-aware placeholders for fulfillment ledger pages**

Create placeholder pages for:

- 收发作业流水
- 包装耗用
- 盘点偏差
- 作业准确度
- 仓网效能

Each page must include:

- business definition
- planned dimensions
- planned metrics
- required data sources
- contract readiness status

- [ ] **Step 3: Keep the write scope isolated from router and menu files**

Worker-B must not edit:

- `F:/erp/apps/erp-web/src/app/router.tsx`
- `F:/erp/apps/erp-web/src/app/layout/menu.tsx`
- `F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts`

If a new route needs registration, hand the route manifest back to the integrator.

- [ ] **Step 4: Run fulfillment-focused smoke tests**

Run the stats E2E gate and any new unit tests introduced for the fulfillment pages.

- [ ] **Step 5: Deliver worker summary**

Return remaining backend contract gaps for the five placeholder pages so they can be scheduled into Phase 2.

### Task 4: Worker-C Catalog And Inventory Insight Pack

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/insights/catalog/HotSkuPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/catalog/SlowSkuPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/catalog/BasketLinksPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/inventory/StockBalancePage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/inventory/AgeHealthPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/inventory/FlowVelocityPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/inventory/TransferEconomicsPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/inventory/NegativeBalanceAlertPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/SlowMovingAnalysisPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/stats/BestSellerAnalysisPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/reports/ReportsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`

- [ ] **Step 1: Canonicalize existing hot/slow seller pages**

Create new `/insights/catalog/*` pages that wrap or migrate:

- 畅销分析 -> 爆品热度
- 滞销分析 -> 长尾沉积

- [ ] **Step 2: Carve inventory report slices out of the current reports page**

Use the existing `ReportsPage` data to seed or support:

- 收发存总览
- 库龄健康
- 周转流速

If the current API is insufficient for a fully dedicated page, create a domain page that embeds the relevant report section first and documents the contract gap explicitly.

- [ ] **Step 3: Create placeholders for contract-heavy pages**

Create phase-aware placeholders for:

- 连带组合
- 调拨效益
- 负库存预警

These should not fabricate data. They should define dimensions, metrics, and dependencies only.

- [ ] **Step 4: Keep write ownership away from shared API files**

Worker-C may read but must not modify:

- `F:/erp/apps/erp-web/src/api/stats.ts`
- `F:/erp/apps/erp-web/src/api/reports.ts`

Any new query shape should be documented and handed back to the integrator/API worker.

- [ ] **Step 5: Deliver worker summary**

Include which inventory pages can ship in Phase 1 vs which require new backend endpoints.

### Task 5: Worker-D Service And Workbench Pack

**Files:**
- Create: `F:/erp/apps/erp-web/src/pages/insights/service/ReturnsPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/service/CompensationPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/service/ReshipQualityPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/service/TicketEfficiencyPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/workbench/ReportCenterPage.tsx`
- Create: `F:/erp/apps/erp-web/src/pages/insights/workbench/ExportQueuePage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/reports/ReportsPage.tsx`
- Modify: `F:/erp/apps/erp-web/src/pages/exports/ExportJobsPage.tsx`
- Test: `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`

- [ ] **Step 1: Move the existing report/export experience into the new workbench naming**

Create canonical route pages:

- 报表中心
- 导出队列

Use wrappers first so the migration remains low risk.

- [ ] **Step 2: Add service-domain placeholder pages**

Create placeholder pages for:

- 退回闭环
- 补偿看板
- 补发质量
- 工单效率

Each page must define:

- target roles
- business question
- planned dimensions
- planned metrics
- required data sources

- [ ] **Step 3: Preserve observable health information**

If `stats-infra-health` or similar observability surfaces are reused, keep them visible so the migrated insight pages still expose runtime readiness.

- [ ] **Step 4: Run focused workbench smoke tests**

Run the existing report/export-related tests if present, plus the stats observability E2E suite.

- [ ] **Step 5: Deliver worker summary**

List which service pages are safe to expose as placeholders immediately and which should stay hidden behind feature flags.

### Task 6: Integrator-Owned Router Registration, Regression, And Release Gate

**Files:**
- Modify: `F:/erp/apps/erp-web/src/app/router.tsx`
- Modify: `F:/erp/apps/erp-web/src/app/navigation/routeAliases.ts`
- Modify: `F:/erp/apps/erp-web/src/app/layout/routeMeta.ts`
- Modify: `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- Modify: `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`
- Verify: `F:/erp/docs/superpowers/specs/2026-04-16-analysis-menu-industrial-refactor-design.md`

- [ ] **Step 1: Register only the approved Phase 1 canonical routes**

Wire the routes for:

- `/insights`
- `/insights/revenue/overview`
- `/insights/revenue/daily-brief`
- `/insights/revenue/trend-radar`
- `/insights/fulfillment/lead-time`
- `/insights/fulfillment/order-cycle`
- `/insights/catalog/hot-sku`
- `/insights/catalog/slow-sku`
- `/insights/workbench/report-center`
- `/insights/workbench/export-queue`

Do not register Phase 2 routes unless their pages and contracts are ready or intentionally behind a feature flag.

- [ ] **Step 2: Add legacy redirects**

Ensure the old routes still work and redirect with query string and hash preserved.

- [ ] **Step 3: Update navigation and smoke E2E**

Extend:

- `F:/erp/apps/erp-web/e2e/workbench-menu-navigation.spec.ts`
- `F:/erp/apps/erp-web/e2e/task8-stats-observability-sso.spec.ts`

to cover at least:

- opening the `洞察` module from the header
- navigating to the Phase 1 canonical pages
- verifying old `/stats/*` URLs redirect to `/insights/*`
- verifying analytics health widgets remain visible

- [ ] **Step 4: Run the regression suite**

Run:

- `pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.ts`
- `pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts e2e/task8-stats-observability-sso.spec.ts`

Expected: PASS or an explicitly documented list of unrelated failures.

- [ ] **Step 5: Generate parallel task cards for execution**

Before implementation starts in earnest, generate task cards under:

- `F:/erp/docs/ops/tasks/2026-04-16/`

Recommended cards:

- `analysis-integrator-shell.md`
- `analysis-revenue-pack.md`
- `analysis-fulfillment-pack.md`
- `analysis-catalog-inventory-pack.md`
- `analysis-service-workbench-pack.md`
- `analysis-regression-gate.md`

Use:

`node scripts/ops/new_parallel_task.mjs`

and keep each card aligned with the write ownership defined in this plan.

### Task 7: Parallel Dispatch Rules

**Files:**
- Verify only

- [ ] **Step 1: Dispatch one worker per domain pack**

Recommended ownership:

- Integrator: Task 1 + Task 6
- Worker-A: Task 2
- Worker-B: Task 3
- Worker-C: Task 4
- Worker-D: Task 5

- [ ] **Step 2: Enforce forbidden write zones**

No domain worker may modify integrator-owned shared files directly. If a worker needs a shared file change, it must send a route/menu/API manifest back to the integrator.

- [ ] **Step 3: Run boundary validation before merge**

For every worker branch, run:

`node scripts/ops/validate_task_boundary.mjs --task "<task-card>" --mode all`

Expected: `PASS`

- [ ] **Step 4: Merge in dependency order**

Recommended merge order:

1. Integrator shell
2. Revenue pack
3. Fulfillment pack
4. Catalog and inventory pack
5. Service and workbench pack
6. Regression gate updates

- [ ] **Step 5: Record residual Phase 2 backlog**

After Phase 1 merge, explicitly carry forward the contract-heavy pages as a separate backlog instead of silently exposing weak placeholders as finished functionality.
