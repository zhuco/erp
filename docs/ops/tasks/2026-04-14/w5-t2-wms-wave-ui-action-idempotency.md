---
name: W5-T2 波次作业看板与动作幂等保护
goal: 增强波次页面可视化和重复点击幂等保护。
background: 现场高频操作下幂等保护缺失会导致重复执行。
owner: W5-owner
window: W5
domain: wms-outbound-wave
slot: T2
branch: codex/w5-wms-outbound-wave-t2-20260414
worktree: .worktrees/w5-wms-outbound-wave-t2-20260414
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
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w5-t2-wms-wave-ui-action-idempotency.md --mode all
  - cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/wms-wave-main-chain.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
增强波次页面可视化和重复点击幂等保护。

## 业务背景
现场高频操作下幂等保护缺失会导致重复执行。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w5-t2-wms-wave-ui-action-idempotency.md --mode all`
- `cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/wms-wave-main-chain.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
