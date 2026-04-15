---
name: ERP95-5T-04 履约与补货执行闭环
goal: 收口波次、出库、发货确认、库存一致性、采购建议、补货计划、采购单、收货与执行回写，形成从订单驱动到补货执行的履约闭环。
background: 本任务卡吸收原 ERP95-06 与 ERP95-07 的内容，不再把履约链和补货链拆成两张卡片，而是按共享库存和执行状态统一收口。
owner: Codex-5T-04
branch: codex/erp95-5t-04-fulfillment-planning
worktree: .worktrees/erp95-06
action: active
source_plan: docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md
replaces:
  - docs/ops/tasks/2026-04-14/erp95-track-06.md
  - docs/ops/tasks/2026-04-14/erp95-track-07.md
allowed_files:
  - apps/erp-web/src/pages/wms/WmsOutboundsPage.tsx
  - apps/erp-web/src/pages/wms/WmsWavesPage.tsx
  - apps/erp-web/src/pages/wms/WmsShipPage.tsx
  - apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx
  - apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx
  - apps/erp-web/src/pages/procurement/ProcurementRequestsPage.tsx
  - apps/erp-web/src/pages/procurement/ProcurementPlansPage.tsx
  - apps/erp-web/src/pages/procurement/PurchaseOrdersPage.tsx
  - apps/erp-web/src/pages/procurement/PurchaseReceiptsPage.tsx
  - apps/erp-web/src/pages/procurement/PurchaseSuggestionsPage.tsx
  - apps/erp-web/src/pages/planning/ReplenishmentRulesPage.tsx
  - apps/erp-web/src/pages/planning/PurchasePlansPage.tsx
  - apps/erp-web/src/pages/planning/PlanExecutionPage.tsx
  - crates/erp-repo/src/wms.rs
  - crates/erp-repo/src/shipments.rs
  - crates/erp-repo/src/inventory.rs
  - crates/erp-repo/src/atp_inventory.rs
  - crates/erp-api/src/planning.rs
  - crates/erp-repo/src/procurement.rs
  - crates/erp-repo/src/planning.rs
  - crates/erp-repo/src/suppliers.rs
  - crates/erp-api/tests/shipment_confirm.rs
  - crates/erp-api/tests/wave_ship_integration.rs
  - crates/erp-api/tests/inventory_atp_fefo_integration.rs
  - crates/erp-api/tests/wms_labor_slotting_flow.rs
  - crates/erp-api/tests/planning_replenishment_flow.rs
  - crates/erp-repo/tests/procurement_flow.rs
  - apps/erp-web/e2e/wms-wave-main-chain.spec.ts
  - apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts
  - apps/erp-web/e2e/procurement-pages.spec.ts
  - apps/erp-web/e2e/planning-flow.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/**
  - apps/erp-web/src/pages/orders/**
  - apps/erp-web/src/pages/afterSales/**
  - apps/erp-web/src/pages/finance/**
  - package.json
  - scripts/**
  - docs/ops/**
dependencies:
  - 优先吸收 erp95-06 中已完成的页面改动。
  - 若启用 erp95-07 相关推进，必须与库存状态口径保持一致。
  - 通过状态契约消费 OMS 输出，不直接争抢 OMS 文件。
dod:
  - 波次、出库、发货确认、库存一致性形成履约闭环。
  - 采购建议、补货计划、采购单、收货、执行反馈形成补货闭环。
  - 履约和补货相关后端测试、仓储与计划 E2E 具备通过路径。
  - 无越界修改。
test_commands:
  - cargo test -p erp-api --test shipment_confirm --test wave_ship_integration --test inventory_atp_fefo_integration --test wms_labor_slotting_flow --test planning_replenishment_flow -- --nocapture
  - cargo test -p erp-repo --test procurement_flow -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts e2e/procurement-pages.spec.ts e2e/planning-flow.spec.ts --project=chromium
delivery: commit hash + 变更摘要 + 测试结果 + 剩余风险
---

## 任务目标
以共享库存和执行状态为中心，把 WMS 履约链和采购补货链真正打通，避免一边能发货、一边补货逻辑完全脱节。

## 执行步骤
- [ ] Step 1: 盘点 `erp95-06` 与 `erp95-07` 的现有改动，拆开履约问题和补货问题。
- [ ] Step 2: 先跑失败测试，锁定履约一致性、库存口径、补货执行回写缺口。
- [ ] Step 3: 完成波次、出库、发货确认、ATP/FEFO 的履约收口。
- [ ] Step 4: 完成采购建议、采购计划、采购单、收货与执行反馈收口。
- [ ] Step 5: 对齐库存、履约、补货三者的共享状态口径。
- [ ] Step 6: 跑通后端测试与 E2E，并记录无法在本轮扩展的范围。

## 特别注意
- 不扩展 PDA、自动化设备、复杂供应商绩效平台。
- 若发现履约状态与 OMS 不一致，只记录契约问题，不越界改订单文件。
- 若采购计划无法完全闭环，至少保证执行回写和库存口径可解释。
