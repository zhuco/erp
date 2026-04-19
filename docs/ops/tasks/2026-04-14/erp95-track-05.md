> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-03.md。

---
name: ERP95-05 sales and OMS main chain closure
goal: Close the OMS main path across review, exceptions, split and merge, recheck, settlement, and audit persistence.
background: The sales main chain is the most important commercial path in the first release. This track moves OMS from broad scaffolding to executable, auditable, and gateable workflows.
owner: Codex-05
branch: codex/erp95-05-oms
worktree: .worktrees/erp95-05
allowed_files:
  - apps/erp-web/src/pages/orders/OrderReviewPage.tsx
  - apps/erp-web/src/pages/orders/OrderExceptionsPage.tsx
  - apps/erp-web/src/pages/orders/OrderSettlementPage.tsx
  - apps/erp-web/src/pages/orders/OrderSplitMergeLegacyPage.tsx
  - apps/erp-web/src/pages/orders/review-native/OrderReviewMainGrid.tsx
  - apps/erp-web/src/pages/orders/review-native/orderReviewToolbarActions.ts
  - crates/erp-api/src/orders.rs
  - crates/erp-repo/src/orders.rs
  - crates/erp-repo/src/order_reviews.rs
  - crates/erp-repo/src/order_action_logs.rs
  - crates/erp-repo/src/order_status_history.rs
  - crates/erp-api/tests/oms_orders.rs
  - crates/erp-api/tests/order_exception_sla_integration.rs
  - crates/erp-api/tests/order_match_ship_wave_persistence_guard.rs
  - apps/erp-web/e2e/order-processing-flow.spec.ts
  - apps/erp-web/e2e/order-exceptions.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Review, split and merge, and settlement writes must have persistence evidence.
  - Do not remove logic to get tests green.
dod:
  - Review, exceptions, split and merge, recheck, and settlement share a consistent state chain.
  - Operation logs and status history are queryable.
  - oms_orders, order_exception_sla_integration, and order_match_ship_wave_persistence_guard pass.
  - OMS E2E main flow passes.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test oms_orders --test order_exception_sla_integration --test order_match_ship_wave_persistence_guard -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/order-processing-flow.spec.ts e2e/order-exceptions.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Close the OMS main path across review, exceptions, split and merge, recheck, settlement, and audit persistence.

## 涓氬姟鑳屾櫙
The sales main chain is the most important commercial path in the first release. This track moves OMS from broad scaffolding to executable, auditable, and gateable workflows.

## DoD
- Review, exceptions, split and merge, recheck, and settlement share a consistent state chain.
- Operation logs and status history are queryable.
- oms_orders, order_exception_sla_integration, and order_match_ship_wave_persistence_guard pass.
- OMS E2E main flow passes.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Review, split and merge, and settlement writes must have persistence evidence.
- Do not remove logic to get tests green.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test oms_orders --test order_exception_sla_integration --test order_match_ship_wave_persistence_guard -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/order-processing-flow.spec.ts e2e/order-exceptions.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

