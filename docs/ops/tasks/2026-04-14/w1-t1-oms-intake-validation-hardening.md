---
name: W1-T1 订单引入幂等与字段校验加固
goal: 完善建单入口参数校验与幂等约束，降低重复建单和脏数据风险。
background: 订单引入是 OMS 主链路入口，输入质量决定后续审核与结算稳定性。
owner: W1-owner
window: W1
domain: oms-intake
slot: T1
branch: codex/w1-oms-intake-t1-20260414
worktree: .worktrees/w1-oms-intake-t1-20260414
allowed_files:
  - crates/erp-api/src/orders/intake.rs
  - apps/erp-web/src/pages/orders/components/*Query*
  - apps/erp-web/src/api/orders.ts
disallowed_files:
  - crates/erp-api/src/orders/review_control.rs
  - crates/erp-api/src/orders/settlement.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w1-t1-oms-intake-validation-hardening.md --mode all
  - cargo test -p erp-api --test oms_orders -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善建单入口参数校验与幂等约束，降低重复建单和脏数据风险。

## 业务背景
订单引入是 OMS 主链路入口，输入质量决定后续审核与结算稳定性。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w1-t1-oms-intake-validation-hardening.md --mode all`
- `cargo test -p erp-api --test oms_orders -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
