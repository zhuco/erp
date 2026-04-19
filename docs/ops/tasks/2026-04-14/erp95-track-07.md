> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-04.md。

---
name: ERP95-07 procurement and planning execution loop
goal: Connect procurement requests, replenishment planning, purchase orders, receipts, and execution feedback into one loop.
background: Procurement and planning are key support domains for the first commercial release. This track turns plan and suggestion screens into an executable and traceable flow.
owner: Codex-07
branch: codex/erp95-07-procurement-planning
worktree: .worktrees/erp95-07
allowed_files:
  - apps/erp-web/src/pages/procurement/ProcurementRequestsPage.tsx
  - apps/erp-web/src/pages/procurement/ProcurementPlansPage.tsx
  - apps/erp-web/src/pages/procurement/PurchaseOrdersPage.tsx
  - apps/erp-web/src/pages/procurement/PurchaseReceiptsPage.tsx
  - apps/erp-web/src/pages/procurement/PurchaseSuggestionsPage.tsx
  - apps/erp-web/src/pages/planning/ReplenishmentRulesPage.tsx
  - apps/erp-web/src/pages/planning/PurchasePlansPage.tsx
  - apps/erp-web/src/pages/planning/PlanExecutionPage.tsx
  - crates/erp-api/src/planning.rs
  - crates/erp-repo/src/procurement.rs
  - crates/erp-repo/src/planning.rs
  - crates/erp-repo/src/suppliers.rs
  - crates/erp-api/tests/planning_replenishment_flow.rs
  - crates/erp-repo/tests/procurement_flow.rs
  - apps/erp-web/e2e/procurement-pages.spec.ts
  - apps/erp-web/e2e/planning-flow.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Do not stop at UI-only changes; execution write-back is required.
  - Keep compatibility with inventory and WMS source and execution states.
dod:
  - Procurement request, replenishment plan, purchase order, receipt, and execution feedback form one loop.
  - Supplier and planning parameters have minimum audit metadata.
  - planning_replenishment_flow and procurement_flow pass.
  - Procurement and planning E2E tests pass.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test planning_replenishment_flow -- --nocapture
  - cargo test -p erp-repo --test procurement_flow -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/procurement-pages.spec.ts e2e/planning-flow.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Connect procurement requests, replenishment planning, purchase orders, receipts, and execution feedback into one loop.

## 涓氬姟鑳屾櫙
Procurement and planning are key support domains for the first commercial release. This track turns plan and suggestion screens into an executable and traceable flow.

## DoD
- Procurement request, replenishment plan, purchase order, receipt, and execution feedback form one loop.
- Supplier and planning parameters have minimum audit metadata.
- planning_replenishment_flow and procurement_flow pass.
- Procurement and planning E2E tests pass.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Do not stop at UI-only changes; execution write-back is required.
- Keep compatibility with inventory and WMS source and execution states.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test planning_replenishment_flow -- --nocapture`
- `cargo test -p erp-repo --test procurement_flow -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/procurement-pages.spec.ts e2e/planning-flow.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

