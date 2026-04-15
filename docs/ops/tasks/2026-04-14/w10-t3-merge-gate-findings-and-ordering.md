---
name: W10-T3 批次门禁 findings 与合并顺序建议
goal: 输出越界、缺测、锁冲突检查结果及合并顺序建议。
background: W10-T3 仅输出 findings 与合并建议，不做业务代码实现。
owner: W10-owner
window: W10
domain: system-governance-gate
slot: T3
branch: codex/w10-system-governance-gate-t3-20260414
worktree: .worktrees/w10-system-governance-gate-t3-20260414
allowed_files:
  - crates/erp-repo/src/rbac.rs
  - crates/erp-repo/src/permissions.rs
  - crates/erp-repo/src/audit_logs.rs
  - crates/erp-repo/src/operation_logs.rs
  - crates/erp-repo/src/reports.rs
  - apps/erp-web/src/pages/system/**
  - scripts/openapi/**
disallowed_files:
  - crates/erp-repo/src/after_sales*.rs
  - crates/erp-repo/src/wms*.rs
  - crates/erp-repo/src/inventory*.rs
  - crates/erp-repo/src/procurement*.rs
  - crates/erp-repo/src/product*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w10-t3-merge-gate-findings-and-ordering.md --mode all
  - pnpm run repo:verify
  - node scripts/ops/generate_parallel_scorecard.mjs --input docs/ops/tasks/2026-04-14/parallel-scorecard-input.json --output docs/ops/tasks/2026-04-14/parallel-scorecard.md
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
输出越界、缺测、锁冲突检查结果及合并顺序建议。

## 业务背景
W10-T3 仅输出 findings 与合并建议，不做业务代码实现。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w10-t3-merge-gate-findings-and-ordering.md --mode all`
- `pnpm run repo:verify`
- `node scripts/ops/generate_parallel_scorecard.mjs --input docs/ops/tasks/2026-04-14/parallel-scorecard-input.json --output docs/ops/tasks/2026-04-14/parallel-scorecard.md`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
