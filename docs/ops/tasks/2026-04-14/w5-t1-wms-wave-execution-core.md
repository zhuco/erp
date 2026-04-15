---
name: W5-T1 出库波次执行主链路加固
goal: 完善波次执行状态与作业落库字段，保障现场执行稳定。
background: WMS 波次链路是履约核心路径，必须优先保证可追踪。
owner: W5-owner
window: W5
domain: wms-outbound-wave
slot: T1
branch: codex/w5-wms-outbound-wave-t1-20260414
worktree: .worktrees/w5-wms-outbound-wave-t1-20260414
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
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w5-t1-wms-wave-execution-core.md --mode all
  - cargo test -p erp-api --test wave_ship_integration -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善波次执行状态与作业落库字段，保障现场执行稳定。

## 业务背景
WMS 波次链路是履约核心路径，必须优先保证可追踪。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w5-t1-wms-wave-execution-core.md --mode all`
- `cargo test -p erp-api --test wave_ship_integration -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
