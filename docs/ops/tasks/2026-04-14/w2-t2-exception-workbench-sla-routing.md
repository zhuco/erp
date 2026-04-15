---
name: W2-T2 异常工作台分流与 SLA 字段补齐
goal: 补齐异常分流字段与优先级规则，提升异常处理效率。
background: 审核异常需要快速定位与分派，字段缺失会导致处理超时。
owner: W2-owner
window: W2
domain: oms-review-exception
slot: T2
branch: codex/w2-oms-review-exception-t2-20260414
worktree: .worktrees/w2-oms-review-exception-t2-20260414
allowed_files:
  - crates/erp-api/src/orders/review_control.rs
  - crates/erp-api/src/orders/exception_workbench.rs
  - apps/erp-web/src/pages/orders/review-native/**
  - apps/erp-web/src/pages/orders/exceptions-native/**
disallowed_files:
  - crates/erp-api/src/orders/intake.rs
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
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w2-t2-exception-workbench-sla-routing.md --mode all
  - cargo test -p erp-api --test order_exception_sla_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/order-exceptions.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
补齐异常分流字段与优先级规则，提升异常处理效率。

## 业务背景
审核异常需要快速定位与分派，字段缺失会导致处理超时。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w2-t2-exception-workbench-sla-routing.md --mode all`
- `cargo test -p erp-api --test order_exception_sla_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/order-exceptions.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
