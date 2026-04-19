---
name: ERP95-5T-05 售后财务与集成门禁
goal: 收口售后主链、经营会计闭环、观测告警、发布门禁、能力矩阵与 Go/No-Go 证据包，形成最终上线收口轨。
background: 本任务卡吸收原 ERP95-08、ERP95-09、ERP95-10 的内容，把售后、财务、门禁、观测与最终评估合并成一个最终收口任务，避免“业务完成了但无法发布”或“门禁通过了但业务证据不足”。
owner: Codex-5T-05
branch: codex/erp95-5t-05-aftersales-finance-gate
worktree: .worktrees/erp95-08
action: active
source_plan: docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md
replaces:
  - docs/ops/tasks/2026-04-14/erp95-track-08.md
  - docs/ops/tasks/2026-04-14/erp95-track-09.md
  - docs/ops/tasks/2026-04-14/erp95-track-10.md
allowed_files:
  - apps/erp-web/src/pages/afterSales/AfterSalesListPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesReceivePage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesExchangesPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesInspectionsPage.tsx
  - apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx
  - apps/erp-web/src/pages/afterSales/RefundDocsPage.tsx
  - apps/erp-web/src/pages/customers/CustomerServiceTicketsPage.tsx
  - apps/erp-web/src/pages/finance/ArManagementPage.tsx
  - apps/erp-web/src/pages/finance/ApManagementPage.tsx
  - apps/erp-web/src/pages/finance/ReconcilePage.tsx
  - apps/erp-web/src/pages/finance/ExpenseAllocationPage.tsx
  - apps/erp-web/src/pages/finance/ProfitPage.tsx
  - apps/erp-web/src/pages/finance/InvoicesPage.tsx
  - apps/erp-web/src/pages/system/ObservabilityPage.tsx
  - apps/erp-web/src/pages/system/frontendRouteObservability.ts
  - crates/erp-repo/src/after_sales.rs
  - crates/erp-repo/src/after_sales_sla_rules.rs
  - crates/erp-repo/src/after_sales_quality_rules.rs
  - crates/erp-repo/src/after_sales_intercepts.rs
  - crates/erp-repo/src/after_sale_cases.rs
  - crates/erp-api/src/finance_management.rs
  - crates/erp-repo/src/finance.rs
  - crates/erp-repo/src/finance_ar.rs
  - crates/erp-repo/src/finance_ap_plans.rs
  - crates/erp-repo/src/finance_expenses.rs
  - crates/erp-repo/src/finance_costing.rs
  - crates/erp-repo/src/finance_tax.rs
  - package.json
  - .github/workflows/ci.yml
  - scripts/release_gate_backend.mjs
  - scripts/verify_openapi_gate_v2.mjs
  - scripts/verify_repo_baseline.mjs
  - docs/ops/RELEASE_GATE_CHECKLIST_V2.md
  - docs/ops/GO_NO_GO_CHECKLIST_V2.md
  - docs/ops/TASK_COMPLETION_MATRIX_V2.md
  - docs/ops/progress_check.md
  - docs/spec/erp-launch-full-feature-matrix.md
  - infra/observability/prometheus/alerts.yml
  - crates/erp-api/tests/after_sales.rs
  - crates/erp-api/tests/after_sales_state_machine_guard.rs
  - crates/erp-api/tests/after_sales_sla_escalation.rs
  - crates/erp-api/tests/after_sales_exchange_profit_flow.rs
  - crates/erp-api/tests/finance_reconcile.rs
  - crates/erp-api/tests/after_sales_finance_reconcile.rs
  - crates/erp-repo/tests/finance_profit.rs
  - apps/erp-web/e2e/after-sales-workflow.spec.ts
  - apps/erp-web/e2e/after-sales-return-resend-main-chain.spec.ts
  - apps/erp-web/e2e/customer-service-tickets.spec.ts
  - apps/erp-web/e2e/task6-after-sales-logistics-profit.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/**
  - apps/erp-web/src/pages/orders/**
  - apps/erp-web/src/pages/wms/**
  - apps/erp-web/src/pages/procurement/**
  - apps/erp-web/src/pages/planning/**
dependencies:
  - 以 Track-03 的订单状态契约和 Track-04 的履约结果为输入。
  - 优先使用现有售后 go-live 材料与能力矩阵，不另起一套口径。
  - 允许本卡片统一维护门禁脚本、观测页面与最终进度结论。
dod:
  - 售后状态机、逆向流程、客服工单、退款补发形成一条闭环。
  - AR/AP/对账/费用分摊/利润/开票形成首发经营会计闭环。
  - 观测、告警、门禁脚本、进度表与能力矩阵同步更新。
  - repo:verify、release gate、相关测试具备通过路径或明确环境阻塞证据。
  - 形成 Go/No-Go 证据包。
  - 无越界修改。
test_commands:
  - cargo test -p erp-api --test after_sales --test after_sales_state_machine_guard --test after_sales_sla_escalation --test after_sales_exchange_profit_flow --test finance_reconcile --test after_sales_finance_reconcile -- --nocapture
  - cargo test -p erp-repo --test finance_profit -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/after-sales-workflow.spec.ts e2e/after-sales-return-resend-main-chain.spec.ts e2e/customer-service-tickets.spec.ts e2e/task6-after-sales-logistics-profit.spec.ts --project=chromium
  - pnpm run repo:verify
  - pnpm run test:backend:release-gate
  - pnpm --dir apps/erp-web run generate:api
  - pnpm --dir apps/erp-web build
  - pnpm run test:e2e:release-gate
delivery: commit hash + 变更摘要 + 测试结果 + 环境阻塞说明 + Go/No-Go 结论
---

## 任务目标
把售后、财务、观测和发布门禁放到一个统一收口轨中，确保最终不是“局部功能能演示”，而是“整体版本能评审、能放行、能解释为什么还不能放行”。

## 执行步骤
- [ ] Step 1: 盘点 `erp95-08`、`erp95-09`、`erp95-10` 的现有成果和缺口。
- [ ] Step 2: 先跑售后与财务失败测试，锁定状态机、回写、利润口径问题。
- [ ] Step 3: 完成售后主链和经营会计闭环收口。
- [ ] Step 4: 更新观测页、告警规则、门禁脚本和进度矩阵。
- [ ] Step 5: 跑通全局门禁，区分逻辑失败与环境阻塞。
- [ ] Step 6: 输出最终 Go/No-Go 证据包与 95%+ 审慎结论。

## 特别注意
- 本卡片负责最终门禁与证据口径，其他卡片不再各自定义“完成”。
- 不把财务扩展成总账平台，首发只收口经营会计链。
- 任何门禁失败都必须写明是逻辑问题还是环境问题，不能混写。
