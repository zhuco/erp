---
name: W1-T3 订单引入链路回归收口
goal: 形成 W1 批次可合并证据并输出残余风险。
background: T3 负责将 T1/T2 的改动转为可审计、可合并的回归结果。
owner: W1-owner
window: W1
domain: oms-intake
slot: T3
branch: codex/w1-oms-intake-t3-20260414
worktree: .worktrees/w1-oms-intake-t3-20260414
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
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w1-t3-intake-chain-regression-pack.md --mode all
  - cargo test -p erp-api --test oms_orders -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/order-review-query.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
形成 W1 批次可合并证据并输出残余风险。

## 业务背景
T3 负责将 T1/T2 的改动转为可审计、可合并的回归结果。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w1-t3-intake-chain-regression-pack.md --mode all`
- `cargo test -p erp-api --test oms_orders -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/order-review-query.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
