---
name: W10-T1 权限与审计基线巡检
goal: 补齐 RBAC 与审计日志关键校验，保证并行提交可追溯。
background: W10 只做系统治理与门禁，不做跨域业务实现。
owner: W10-owner
window: W10
domain: system-governance-gate
slot: T1
branch: codex/w10-system-governance-gate-t1-20260414
worktree: .worktrees/w10-system-governance-gate-t1-20260414
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
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w10-t1-rbac-audit-baseline-check.md --mode all
  - cargo test -p erp-api --test system_audit_reports_integration -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
补齐 RBAC 与审计日志关键校验，保证并行提交可追溯。

## 业务背景
W10 只做系统治理与门禁，不做跨域业务实现。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w10-t1-rbac-audit-baseline-check.md --mode all`
- `cargo test -p erp-api --test system_audit_reports_integration -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
