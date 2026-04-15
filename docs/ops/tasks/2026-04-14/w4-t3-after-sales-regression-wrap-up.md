---
name: W4-T3 售后主链路回归收口
goal: 完成售后窗口批次回归并提交残余风险。
background: T3 负责售后域合并前收口，降低跨域回归风险。
owner: W4-owner
window: W4
domain: after-sales
slot: T3
branch: codex/w4-after-sales-t3-20260414
worktree: .worktrees/w4-after-sales-t3-20260414
allowed_files:
  - crates/erp-repo/src/after_sales*.rs
  - crates/erp-api/tests/after_sales*.rs
  - apps/erp-web/src/pages/afterSales/**
disallowed_files:
  - crates/erp-repo/src/finance*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w4-t3-after-sales-regression-wrap-up.md --mode all
  - cargo test -p erp-api --test after_sales_finance_reconcile -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/after-sales-return-resend-main-chain.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完成售后窗口批次回归并提交残余风险。

## 业务背景
T3 负责售后域合并前收口，降低跨域回归风险。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w4-t3-after-sales-regression-wrap-up.md --mode all`
- `cargo test -p erp-api --test after_sales_finance_reconcile -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/after-sales-return-resend-main-chain.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
