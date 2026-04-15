---
name: W7-T1 采购到货入库主链路打通
goal: 完善采购到货与入库确认链路，提升入库可追溯性。
background: 采购与入库链路断裂会直接影响库存与履约准确性。
owner: W7-owner
window: W7
domain: procurement-inbound
slot: T1
branch: codex/w7-procurement-inbound-t1-20260414
worktree: .worktrees/w7-procurement-inbound-t1-20260414
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
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w7-t1-procurement-inbound-main-chain.md --mode all
  - cargo test -p erp-api --test planning_replenishment_flow -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善采购到货与入库确认链路，提升入库可追溯性。

## 业务背景
采购与入库链路断裂会直接影响库存与履约准确性。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w7-t1-procurement-inbound-main-chain.md --mode all`
- `cargo test -p erp-api --test planning_replenishment_flow -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
