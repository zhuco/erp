---
name: W10-T2 OpenAPI 门禁脚本与校验加固
goal: 增强 OpenAPI 相关门禁脚本可读性与一致性校验。
background: OpenAPI 文件是并行冲突高发点，需要治理窗口集中收口。
owner: W10-owner
window: W10
domain: system-governance-gate
slot: T2
branch: codex/w10-system-governance-gate-t2-20260414
worktree: .worktrees/w10-system-governance-gate-t2-20260414
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
shared_lock_targets:
  - scripts/openapi/lib/openapi_tools.mjs
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1 核心字段契约已稳定
  - 已完成共享锁登记并持锁编辑
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w10-t2-openapi-gate-hardening.md --mode all
  - pnpm run openapi:bundle
  - pnpm run repo:verify:openapi
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
增强 OpenAPI 相关门禁脚本可读性与一致性校验。

## 业务背景
OpenAPI 文件是并行冲突高发点，需要治理窗口集中收口。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定
- 已完成共享锁登记并持锁编辑

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w10-t2-openapi-gate-hardening.md --mode all`
- `pnpm run openapi:bundle`
- `pnpm run repo:verify:openapi`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
