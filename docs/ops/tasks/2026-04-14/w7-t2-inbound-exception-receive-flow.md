---
name: W7-T2 入库异常拒收与补录流程加固
goal: 补齐拒收、补录与二次入库流程，降低异常积压。
background: 异常入库是仓采协同高风险环节，需要独立收敛。
owner: W7-owner
window: W7
domain: procurement-inbound
slot: T2
branch: codex/w7-procurement-inbound-t2-20260414
worktree: .worktrees/w7-procurement-inbound-t2-20260414
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
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w7-t2-inbound-exception-receive-flow.md --mode all
  - cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/inbound-flow.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
补齐拒收、补录与二次入库流程，降低异常积压。

## 业务背景
异常入库是仓采协同高风险环节，需要独立收敛。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w7-t2-inbound-exception-receive-flow.md --mode all`
- `cargo test -p erp-api --test wms_inbound_inventory_workflow_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/inbound-flow.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
