---
name: W1-T2 查询组件与建单 API 参数对齐
goal: 统一前端查询参数与后端建单字段语义，避免字段漂移。
background: 并行开发中，查询条件与 API 字段不一致会造成大量误判与返工。
owner: W1-owner
window: W1
domain: oms-intake
slot: T2
branch: codex/w1-oms-intake-t2-20260414
worktree: .worktrees/w1-oms-intake-t2-20260414
allowed_files:
  - crates/erp-api/src/orders/intake.rs
  - apps/erp-web/src/pages/orders/components/*Query*
  - apps/erp-web/src/api/orders.ts
disallowed_files:
  - crates/erp-api/src/orders/review_control.rs
  - crates/erp-api/src/orders/settlement.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w1-t2-query-and-api-parameter-alignment.md --mode all
  - pnpm --filter erp-web test:unit
  - cargo test -p erp-api --test oms_orders -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
统一前端查询参数与后端建单字段语义，避免字段漂移。

## 业务背景
并行开发中，查询条件与 API 字段不一致会造成大量误判与返工。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w1-t2-query-and-api-parameter-alignment.md --mode all`
- `pnpm --filter erp-web test:unit`
- `cargo test -p erp-api --test oms_orders -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
