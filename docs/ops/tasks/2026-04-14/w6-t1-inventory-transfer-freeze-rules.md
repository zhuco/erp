---
name: W6-T1 库存调拨与冻结规则加固
goal: 完善库存调拨状态与冻结规则，降低跨仓错账风险。
background: 库存治理窗口优先保障可用量与调拨一致性。
owner: W6-owner
window: W6
domain: inventory-governance
slot: T1
branch: codex/w6-inventory-governance-t1-20260414
worktree: .worktrees/w6-inventory-governance-t1-20260414
allowed_files:
  - crates/erp-repo/src/inventory*.rs
  - crates/erp-repo/src/stock_*.rs
  - apps/erp-web/src/pages/inventory/**
disallowed_files:
  - crates/erp-repo/src/wms*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w6-t1-inventory-transfer-freeze-rules.md --mode all
  - cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善库存调拨状态与冻结规则，降低跨仓错账风险。

## 业务背景
库存治理窗口优先保障可用量与调拨一致性。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w6-t1-inventory-transfer-freeze-rules.md --mode all`
- `cargo test -p erp-api --test inventory_atp_fefo_integration -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
