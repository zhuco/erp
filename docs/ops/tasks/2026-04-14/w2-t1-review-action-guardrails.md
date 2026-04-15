---
name: W2-T1 审核动作守卫与状态约束加固
goal: 完善审核动作前置条件与状态迁移限制，防止非法流转。
background: 审核链路是订单执行中枢，守卫薄弱会放大异常处置成本。
owner: W2-owner
window: W2
domain: oms-review-exception
slot: T1
branch: codex/w2-oms-review-exception-t1-20260414
worktree: .worktrees/w2-oms-review-exception-t1-20260414
allowed_files:
  - crates/erp-api/src/orders/review_control.rs
  - crates/erp-api/src/orders/exception_workbench.rs
  - apps/erp-web/src/pages/orders/review-native/**
  - apps/erp-web/src/pages/orders/exceptions-native/**
disallowed_files:
  - crates/erp-api/src/orders/intake.rs
  - crates/erp-api/src/orders/settlement.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w2-t1-review-action-guardrails.md --mode all
  - cargo test -p erp-api --test oms_orders -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善审核动作前置条件与状态迁移限制，防止非法流转。

## 业务背景
审核链路是订单执行中枢，守卫薄弱会放大异常处置成本。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w2-t1-review-action-guardrails.md --mode all`
- `cargo test -p erp-api --test oms_orders -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
