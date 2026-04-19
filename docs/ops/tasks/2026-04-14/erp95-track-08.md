> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-05.md。

---
name: ERP95-08 customer service after-sales and reverse flow
goal: Close the after-sales main flow across tickets, SLA, reverse receipt, inspection, resend, and platform case aggregation.
background: The after-sales domain is already close to conditional go. This track finishes the operational loop between customer service, SLA, reverse fulfillment, and platform after-sales evidence.
owner: Codex-08
branch: codex/erp95-08-aftersales
worktree: .worktrees/erp95-08
allowed_files:
  - apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesExchangesPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesInspectionsPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx
  - apps/erp-web/src/pages/afterSales/RefundDocsPage.tsx
  - apps/erp-web/src/pages/customers/CustomerServiceTicketsPage.tsx
  - crates/erp-repo/src/after_sales.rs
  - crates/erp-repo/src/after_sales_sla_rules.rs
  - crates/erp-repo/src/after_sales_quality_rules.rs
  - crates/erp-repo/src/after_sales_intercepts.rs
  - crates/erp-repo/src/after_sale_cases.rs
  - crates/erp-api/tests/after_sales.rs
  - crates/erp-api/tests/after_sales_state_machine_guard.rs
  - crates/erp-api/tests/after_sales_sla_escalation.rs
  - crates/erp-api/tests/after_sales_exchange_profit_flow.rs
  - apps/erp-web/e2e/after-sales-workflow.spec.ts
  - apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts
  - apps/erp-web/e2e/customer-service-tickets.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Keep the gate defined in docs/reports/aftersales-go-live-gate.md aligned with the implementation.
  - Do not trade away after-sales finance linkage or SLA fields for UI-only progress.
dod:
  - After-sales state machine, reverse flow, SLA, and ticket handling form one loop.
  - Refund docs, resend, and platform case aggregation keep log evidence.
  - Backend after-sales tests pass.
  - After-sales and customer service E2E tests pass.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test after_sales --test after_sales_state_machine_guard --test after_sales_sla_escalation --test after_sales_exchange_profit_flow -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/after-sales-workflow.spec.ts e2e/after-sales-return-resend-main-chain.spec.ts e2e/customer-service-tickets.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Close the after-sales main flow across tickets, SLA, reverse receipt, inspection, resend, and platform case aggregation.

## 涓氬姟鑳屾櫙
The after-sales domain is already close to conditional go. This track finishes the operational loop between customer service, SLA, reverse fulfillment, and platform after-sales evidence.

## DoD
- After-sales state machine, reverse flow, SLA, and ticket handling form one loop.
- Refund docs, resend, and platform case aggregation keep log evidence.
- Backend after-sales tests pass.
- After-sales and customer service E2E tests pass.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Keep the gate defined in docs/reports/aftersales-go-live-gate.md aligned with the implementation.
- Do not trade away after-sales finance linkage or SLA fields for UI-only progress.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test after_sales --test after_sales_state_machine_guard --test after_sales_sla_escalation --test after_sales_exchange_profit_flow -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/after-sales-workflow.spec.ts e2e/after-sales-return-resend-main-chain.spec.ts e2e/customer-service-tickets.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

