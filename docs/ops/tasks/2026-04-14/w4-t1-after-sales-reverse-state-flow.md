---
name: W4-T1 售后退换补发状态流转加固
goal: 补齐售后状态流转规则与动作限制，避免逆向流程串单。
background: 售后逆向是重点链路，需先稳定状态机后再扩展页面能力。
owner: W4-owner
window: W4
domain: after-sales
slot: T1
branch: codex/w4-after-sales-t1-20260414
worktree: .worktrees/w4-after-sales-t1-20260414
allowed_files:
  - crates/erp-repo/src/after_sales*.rs
  - crates/erp-api/tests/after_sales*.rs
  - apps/erp-web/src/pages/afterSales/**
disallowed_files:
  - crates/erp-repo/src/finance*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w4-t1-after-sales-reverse-state-flow.md --mode all
  - cargo test -p erp-api --test after_sales_state_machine_guard -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
补齐售后状态流转规则与动作限制，避免逆向流程串单。

## 业务背景
售后逆向是重点链路，需先稳定状态机后再扩展页面能力。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w4-t1-after-sales-reverse-state-flow.md --mode all`
- `cargo test -p erp-api --test after_sales_state_machine_guard -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
