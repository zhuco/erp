> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-04.md。

---
name: ERP95-06 WMS inventory and fulfillment consistency
goal: Close the fulfillment chain across ATP, outbound, wave execution, shipment confirmation, and inventory governance views.
background: WMS coverage is broad, but industrial readiness depends on consistent state transitions, replayability, and persistence evidence across the fulfillment main chain.
owner: Codex-06
branch: codex/erp95-06-wms
worktree: .worktrees/erp95-06
allowed_files:
  - apps/erp-web/src/pages/wms/WmsOutboundsPage.tsx
  - apps/erp-web/src/pages/wms/WmsWavesPage.tsx
  - apps/erp-web/src/pages/wms/WmsShipPage.tsx
  - apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx
  - apps/erp-web/src/pages/inventory/InventorySummaryPage.tsx
  - crates/erp-repo/src/wms.rs
  - crates/erp-repo/src/shipments.rs
  - crates/erp-repo/src/inventory.rs
  - crates/erp-repo/src/atp_inventory.rs
  - crates/erp-api/tests/shipment_confirm.rs
  - crates/erp-api/tests/wave_ship_integration.rs
  - crates/erp-api/tests/inventory_atp_fefo_integration.rs
  - crates/erp-api/tests/wms_labor_slotting_flow.rs
  - apps/erp-web/e2e/wms-wave-main-chain.spec.ts
  - apps/erp-web/e2e/outbound-fulfillment-flow.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Stay compatible with the OMS status model without editing OMS-owned files.
  - Keep evidence for partial failure, retry, and replay.
dod:
  - Wave, outbound, shipment confirmation, and ATP behavior is consistent.
  - Partial failure, retry, and replay have state and log evidence.
  - shipment_confirm, wave_ship_integration, inventory_atp_fefo_integration, and wms_labor_slotting_flow pass.
  - WMS E2E main flow passes.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test shipment_confirm --test wave_ship_integration --test inventory_atp_fefo_integration --test wms_labor_slotting_flow -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Close the fulfillment chain across ATP, outbound, wave execution, shipment confirmation, and inventory governance views.

## 涓氬姟鑳屾櫙
WMS coverage is broad, but industrial readiness depends on consistent state transitions, replayability, and persistence evidence across the fulfillment main chain.

## DoD
- Wave, outbound, shipment confirmation, and ATP behavior is consistent.
- Partial failure, retry, and replay have state and log evidence.
- shipment_confirm, wave_ship_integration, inventory_atp_fefo_integration, and wms_labor_slotting_flow pass.
- WMS E2E main flow passes.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Stay compatible with the OMS status model without editing OMS-owned files.
- Keep evidence for partial failure, retry, and replay.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test shipment_confirm --test wave_ship_integration --test inventory_atp_fefo_integration --test wms_labor_slotting_flow -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/wms-wave-main-chain.spec.ts e2e/outbound-fulfillment-flow.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

