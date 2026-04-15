---
name: W3-T3 结算对账回归与审计收口
goal: 完成结算与对账批次回归并提供审计证据。
background: T3 聚合 W3 变更，输出主控可直接判定的合并依据。
owner: W3-owner
window: W3
domain: oms-settlement-finance
slot: T3
branch: codex/w3-oms-settlement-finance-t3-20260414
worktree: .worktrees/w3-oms-settlement-finance-t3-20260414
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
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w3-t3-settlement-audit-regression-pack.md --mode all
  - cargo test -p erp-api --test finance_reconcile -- --nocapture
  - cargo test -p erp-api --test system_audit_reports_integration -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完成结算与对账批次回归并提供审计证据。

## 业务背景
T3 聚合 W3 变更，输出主控可直接判定的合并依据。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w3-t3-settlement-audit-regression-pack.md --mode all`
- `cargo test -p erp-api --test finance_reconcile -- --nocapture`
- `cargo test -p erp-api --test system_audit_reports_integration -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
