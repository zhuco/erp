> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-05.md。

---
name: ERP95-09 finance reconciliation invoicing and business analysis
goal: Close AR, AP, reconciliation, cost allocation, profit reporting, invoicing, and after-sales finance linkage.
background: Finance remains one of the largest gaps between the current project and industrial commercial delivery. This track focuses on the first-release management accounting loop, not a full general ledger.
owner: Codex-09
branch: codex/erp95-09-finance
worktree: .worktrees/erp95-09
allowed_files:
  - apps/erp-web/src/pages/finance/ArManagementPage.tsx
  - apps/erp-web/src/pages/finance/ApManagementPage.tsx
  - apps/erp-web/src/pages/finance/ReconcilePage.tsx
  - apps/erp-web/src/pages/finance/ExpenseAllocationPage.tsx
  - apps/erp-web/src/pages/finance/ProfitPage.tsx
  - apps/erp-web/src/pages/finance/InvoicesPage.tsx
  - crates/erp-api/src/finance_management.rs
  - crates/erp-repo/src/finance.rs
  - crates/erp-repo/src/finance_ar.rs
  - crates/erp-repo/src/finance_ap_plans.rs
  - crates/erp-repo/src/finance_expenses.rs
  - crates/erp-repo/src/finance_costing.rs
  - crates/erp-repo/src/finance_tax.rs
  - crates/erp-api/tests/finance_reconcile.rs
  - crates/erp-api/tests/after_sales_finance_reconcile.rs
  - crates/erp-repo/tests/finance_profit.rs
  - apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Keep scope inside the first-release management accounting loop and do not expand into general ledger work.
  - Stay compatible with the after-sales finance linkage from Track-08.
dod:
  - AR, AP, reconciliation, expenses, profit, and invoicing form a minimum executable loop.
  - Finance views can reference business documents and difference reasons.
  - finance_reconcile, after_sales_finance_reconcile, and finance_profit pass.
  - Business analysis E2E passes.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test finance_reconcile --test after_sales_finance_reconcile -- --nocapture
  - cargo test -p erp-repo --test finance_profit -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Close AR, AP, reconciliation, cost allocation, profit reporting, invoicing, and after-sales finance linkage.

## 涓氬姟鑳屾櫙
Finance remains one of the largest gaps between the current project and industrial commercial delivery. This track focuses on the first-release management accounting loop, not a full general ledger.

## DoD
- AR, AP, reconciliation, expenses, profit, and invoicing form a minimum executable loop.
- Finance views can reference business documents and difference reasons.
- finance_reconcile, after_sales_finance_reconcile, and finance_profit pass.
- Business analysis E2E passes.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Keep scope inside the first-release management accounting loop and do not expand into general ledger work.
- Stay compatible with the after-sales finance linkage from Track-08.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test finance_reconcile --test after_sales_finance_reconcile -- --nocapture`
- `cargo test -p erp-repo --test finance_profit -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

