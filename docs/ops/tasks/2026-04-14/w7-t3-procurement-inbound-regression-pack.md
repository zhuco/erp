---
name: W7-T3 采购入库回归与权限收口
goal: 完成采购入库批次回归并提交标准回执。
background: T3 用于确认采购域变更具备最小权限与主链路验证。
owner: W7-owner
window: W7
domain: procurement-inbound
slot: T3
branch: codex/w7-procurement-inbound-t3-20260414
worktree: .worktrees/w7-procurement-inbound-t3-20260414
allowed_files:
  - crates/erp-repo/src/procurement*.rs
  - crates/erp-repo/src/inbounds*.rs
  - apps/erp-web/src/pages/procurement/**
  - apps/erp-web/src/pages/inbounds/**
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
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w7-t3-procurement-inbound-regression-pack.md --mode all
  - cargo test -p erp-api --test procurement_permissions -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/procurement-pages.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完成采购入库批次回归并提交标准回执。

## 业务背景
T3 用于确认采购域变更具备最小权限与主链路验证。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w7-t3-procurement-inbound-regression-pack.md --mode all`
- `cargo test -p erp-api --test procurement_permissions -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/procurement-pages.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
