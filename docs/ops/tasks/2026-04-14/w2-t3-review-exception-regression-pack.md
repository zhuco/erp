---
name: W2-T3 审核与异常联调回归收口
goal: 完成审核与异常处置联调回归，输出可合并回执。
background: T3 负责 W2 批次收敛，保证审核链路变更可控上线。
owner: W2-owner
window: W2
domain: oms-review-exception
slot: T3
branch: codex/w2-oms-review-exception-t3-20260414
worktree: .worktrees/w2-oms-review-exception-t3-20260414
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
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w2-t3-review-exception-regression-pack.md --mode all
  - cargo test -p erp-api --test order_exception_sla_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/order-exceptions.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完成审核与异常处置联调回归，输出可合并回执。

## 业务背景
T3 负责 W2 批次收敛，保证审核链路变更可控上线。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w2-t3-review-exception-regression-pack.md --mode all`
- `cargo test -p erp-api --test order_exception_sla_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/order-exceptions.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
