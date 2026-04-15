---
name: W3-T2 财务字段口径与对账映射对齐
goal: 统一结算与财务字段映射，降低对账偏差。
background: 结算域与财务域并行开发时，字段口径一致性是高风险点。
owner: W3-owner
window: W3
domain: oms-settlement-finance
slot: T2
branch: codex/w3-oms-settlement-finance-t2-20260414
worktree: .worktrees/w3-oms-settlement-finance-t2-20260414
allowed_files:
  - crates/erp-api/src/orders/settlement.rs
  - crates/erp-api/src/finance_management.rs
  - crates/erp-repo/src/finance*.rs
  - apps/erp-web/src/pages/orders/OrderSettlementPage.tsx
disallowed_files:
  - crates/erp-api/src/orders/review_control.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w3-t2-finance-reconcile-field-alignment.md --mode all
  - cargo test -p erp-api --test finance_reconcile -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/order-processing-flow.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
统一结算与财务字段映射，降低对账偏差。

## 业务背景
结算域与财务域并行开发时，字段口径一致性是高风险点。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w3-t2-finance-reconcile-field-alignment.md --mode all`
- `cargo test -p erp-api --test finance_reconcile -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/order-processing-flow.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
