---
name: W5-T3 波次履约批次回归收口
goal: 形成 W5 批次回归证据并输出可合并回执。
background: T3 汇总出库波次关键证据，支撑主控判断合并时机。
owner: W5-owner
window: W5
domain: wms-outbound-wave
slot: T3
branch: codex/w5-wms-outbound-wave-t3-20260414
worktree: .worktrees/w5-wms-outbound-wave-t3-20260414
allowed_files:
  - crates/erp-repo/src/wms*.rs
  - crates/erp-api/tests/wms_*.rs
  - apps/erp-web/src/pages/wms/**
disallowed_files:
  - crates/erp-repo/src/finance*.rs
  - crates/erp-repo/src/inventory*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w5-t3-wms-wave-regression-pack.md --mode all
  - cargo test -p erp-api --test wave_ship_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/outbound-fulfillment-flow.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
形成 W5 批次回归证据并输出可合并回执。

## 业务背景
T3 汇总出库波次关键证据，支撑主控判断合并时机。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w5-t3-wms-wave-regression-pack.md --mode all`
- `cargo test -p erp-api --test wave_ship_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/outbound-fulfillment-flow.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
