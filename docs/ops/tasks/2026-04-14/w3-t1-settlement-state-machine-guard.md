---
name: W3-T1 结算状态机与单据幂等约束
goal: 完善结算状态迁移守卫与单据幂等，避免重复结算。
background: 结算链路直接影响财务准确性，优先稳定状态机。
owner: W3-owner
window: W3
domain: oms-settlement-finance
slot: T1
branch: codex/w3-oms-settlement-finance-t1-20260414
worktree: .worktrees/w3-oms-settlement-finance-t1-20260414
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
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w3-t1-settlement-state-machine-guard.md --mode all
  - cargo test -p erp-api --test finance_reconcile -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善结算状态迁移守卫与单据幂等，避免重复结算。

## 业务背景
结算链路直接影响财务准确性，优先稳定状态机。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w3-t1-settlement-state-machine-guard.md --mode all`
- `cargo test -p erp-api --test finance_reconcile -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
