# 工业 ERP/WMS 剩余任务块完成矩阵（2026-04-09）

## 目的

将 `docs/superpowers/plans/2026-04-08-industrial-erp-wms-gap-closure-remaining-roadmap.md` 中 Task1~Task9 的最小可验证闭环固化为可审计清单，便于发布前逐项核对。

## 完成矩阵

| 任务块 | 当前状态 | 最小后端链路（示例） | 前端 / E2E（示例） | 关键验证命令 |
| --- | --- | --- | --- | --- |
| Task1 平台接入生产化 | 已完成（最小闭环） | `crates/erp-api/tests/platform_oauth_webhook_integration.rs`、`/v1/tenants/{tenant_id}/system/integrations/shops` | `apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx`、`apps/erp-web/e2e/system-integrations.spec.ts` | `cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture` |
| Task2 OMS 异常中台 | 已完成（最小闭环） | `crates/erp-api/tests/order_exception_sla_integration.rs`、`/v1/tenants/{tenant_id}/oms/orders/exceptions/*` | `apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx`、`apps/erp-web/e2e/release-gate-main-chain.spec.ts` | `cargo test -p erp-api --test order_exception_sla_integration -- --nocapture` |
| Task3 WMS 现场执行 | 已完成（最小闭环） | `crates/erp-api/tests/wms_labor_slotting_flow.rs`、`/v1/tenants/{tenant_id}/wms/labor/tasks` | `apps/erp-web/src/pages/wms/*`、`apps/erp-web/e2e/release-gate-main-chain.spec.ts` | `cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture` |
| Task4 库存承诺 / FEFO / 主数据治理 | 已完成（最小闭环） | `crates/erp-api/tests/inventory_atp_fefo_integration.rs`、`/v1/tenants/{tenant_id}/inventory/atp` | `apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx`、`apps/erp-web/src/pages/products/ProductPackagingSpecsPage.tsx` | `cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture` |
| Task5 计划补货与采购调拨建议 | 已完成（最小闭环） | `crates/erp-api/tests/planning_replenishment_flow.rs`、`/v1/tenants/{tenant_id}/planning/*` | `apps/erp-web/src/pages/planning/*`、`apps/erp-web/e2e/release-gate-cross-chain.spec.ts` | `cargo test -p erp-api --test planning_replenishment_flow -- --nocapture` |
| Task6 售后逆向与利润经营 | 已完成（最小闭环） | `crates/erp-api/tests/after_sales_exchange_profit_flow.rs`、`/v1/tenants/{tenant_id}/after-sales/*`、`/v1/tenants/{tenant_id}/fin/profit` | `apps/erp-web/src/pages/afterSales/*`、`apps/erp-web/src/pages/finance/ProfitPage.tsx` | `cargo test -p erp-api --test after_sales_exchange_profit_flow -- --nocapture` |
| Task7 系统治理二阶段（Camunda） | 已完成（最小闭环） | `crates/erp-api/tests/system_governance_workflow_integration.rs`、`/v1/tenants/{tenant_id}/system/workflows` | `apps/erp-web/src/pages/system/SystemWorkflowsPage.tsx`、`apps/erp-web/e2e/system-governance.spec.ts` | `cargo test -p erp-api --test system_governance_workflow_integration -- --nocapture` |
| Task8 统计分析 / SSO / 可观测 | 已完成（最小闭环） | `crates/erp-api/tests/stats_clickhouse_sso_smoke.rs`、`/v1/tenants/{tenant_id}/stats/health` | `apps/erp-web/src/pages/stats/*`、`apps/erp-web/e2e/task8-stats-observability-sso.spec.ts` | `cargo test -p erp-api --test stats_clickhouse_sso_smoke -- --nocapture` |
| Task9 非功能门禁 | 已完成（门禁固化） | `scripts/release_gate_backend.mjs`、`scripts/verify_openapi_gate.mjs`、`.github/workflows/ci.yml` | `apps/erp-web/e2e/release-gate-main-chain.spec.ts`、`apps/erp-web/e2e/release-gate-cross-chain.spec.ts` | `pnpm run release:gate:full` |

## 当前门禁口径（发布前）

1. `pnpm run repo:verify`
2. `pnpm run test:backend:release-gate`
3. `pnpm --dir apps/erp-web run generate:api`
4. `pnpm --dir apps/erp-web build`
5. `pnpm run test:e2e:release-gate`
6. `pnpm run release:gate:full`

## 本次增补（2026-04-09）

- 扩展 `scripts/verify_openapi_gate.mjs`，将 Task1~Task6 关键端点纳入契约门禁：
  - `/v1/tenants/{tenant_id}/system/integrations/shops`
  - `/v1/tenants/{tenant_id}/oms/orders/exceptions/stats`
  - `/v1/tenants/{tenant_id}/wms/labor/tasks`
  - `/v1/tenants/{tenant_id}/wms/slotting/suggestions`
  - `/v1/tenants/{tenant_id}/inventory/atp`
  - `/v1/tenants/{tenant_id}/inventory/reconciliation-center/check`
  - `/v1/tenants/{tenant_id}/planning/forecasts`
  - `/v1/tenants/{tenant_id}/planning/purchase-plans/generate`
  - `/v1/tenants/{tenant_id}/after-sales/{after_sales_id}/exchange`
  - `/v1/tenants/{tenant_id}/fin/profit`
- 补齐 OpenAPI 缺失的错误响应契约（`400/401/403`）：
  - `GET /v1/tenants/{tenant_id}/wms/labor/tasks`
  - `GET /v1/tenants/{tenant_id}/wms/slotting/suggestions`
  - `GET /v1/tenants/{tenant_id}/fin/profit`
- 新增 `apps/erp-web/scripts/run_release_gate_e2e.mjs`：
  - release-gate E2E 每次自动选择空闲端口执行，规避固定端口残留导致的连接拒绝。
- 新增前端分包规则与单测：
  - `apps/erp-web/src/build/manualChunks.ts`
  - `apps/erp-web/src/build/manualChunks.test.ts`
  - `apps/erp-web/vite.config.ts` 接入 `manualChunks`（当前仅对 `xlsx/tabulator` 重型依赖拆包）。

## 本次增补（2026-04-10）
- `apps/erp-web/src/app/router.tsx` 增量接入路由级懒加载（system/stats/planning/marketing/finance 关键页面），并统一提供 `route-page-loading` 回退占位。
- 前端构建产物中上述页面已按路由切分为独立 chunk（如 `SystemIntegrationsPage-*`、`PlanExecutionPage-*`、`ProfitPage-*`、`SalesStatsPage-*`）。
- `release-gate` 全链路命令复验通过：`pnpm run release:gate:full`。
