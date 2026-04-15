---
name: W4-T2 售后页面操作约束与可视化补齐
goal: 确保售后页面按钮行为与后端状态约束一致。
background: 前后端语义不一致会导致人工误操作与工单反复。
owner: W4-owner
window: W4
domain: after-sales
slot: T2
branch: codex/w4-after-sales-t2-20260414
worktree: .worktrees/w4-after-sales-t2-20260414
allowed_files:
  - crates/erp-repo/src/after_sales*.rs
  - crates/erp-api/tests/after_sales*.rs
  - apps/erp-web/src/pages/afterSales/**
disallowed_files:
  - crates/erp-repo/src/finance*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w4-t2-after-sales-ui-operation-consistency.md --mode all
  - cargo test -p erp-api --test after_sales -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/after-sales-workflow.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
确保售后页面按钮行为与后端状态约束一致。

## 业务背景
前后端语义不一致会导致人工误操作与工单反复。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w4-t2-after-sales-ui-operation-consistency.md --mode all`
- `cargo test -p erp-api --test after_sales -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/after-sales-workflow.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
