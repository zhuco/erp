---
name: W6-T2 盘点差异闭环与调拨联动
goal: 补齐盘点差异处理流程并联动调拨修正动作。
background: 盘点差异是库存治理高频问题，需要建立闭环。
owner: W6-owner
window: W6
domain: inventory-governance
slot: T2
branch: codex/w6-inventory-governance-t2-20260414
worktree: .worktrees/w6-inventory-governance-t2-20260414
allowed_files:
  - crates/erp-repo/src/inventory*.rs
  - crates/erp-repo/src/stock_*.rs
  - apps/erp-web/src/pages/inventory/**
disallowed_files:
  - crates/erp-repo/src/wms*.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w6-t2-stock-count-difference-loop.md --mode all
  - cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/inventory-governance-packaging.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
补齐盘点差异处理流程并联动调拨修正动作。

## 业务背景
盘点差异是库存治理高频问题，需要建立闭环。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w6-t2-stock-count-difference-loop.md --mode all`
- `cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/inventory-governance-packaging.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
