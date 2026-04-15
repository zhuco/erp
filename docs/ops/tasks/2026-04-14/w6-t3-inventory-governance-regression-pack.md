---
name: W6-T3 库存治理回归与风险收口
goal: 输出库存窗口批次回归证据与残余风险。
background: T3 聚合 W6 变更，确保库存域并行提交可控。
owner: W6-owner
window: W6
domain: inventory-governance
slot: T3
branch: codex/w6-inventory-governance-t3-20260414
worktree: .worktrees/w6-inventory-governance-t3-20260414
allowed_files:
  - crates/erp-repo/src/inventory*.rs
  - crates/erp-repo/src/stock_*.rs
  - apps/erp-web/src/pages/inventory/**
disallowed_files:
  - crates/erp-repo/src/wms*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w6-t3-inventory-governance-regression-pack.md --mode all
  - cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/inventory.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
输出库存窗口批次回归证据与残余风险。

## 业务背景
T3 聚合 W6 变更，确保库存域并行提交可控。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w6-t3-inventory-governance-regression-pack.md --mode all`
- `cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/inventory.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
