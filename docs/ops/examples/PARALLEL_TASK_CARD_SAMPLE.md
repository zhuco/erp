---
name: OMS 审核规则收敛示例
goal: 统一订单审核与例外处理动作规则
background: 用于降低审核链路回归并提升主链路稳定性
owner: W2-owner
window: W2
domain: oms-review-exception
slot: T1
branch: codex/oms-review-rules-20260414
worktree: .worktrees/oms-review-rules-20260414
allowed_files:
  - crates/erp-api/src/orders/review_control.rs
  - crates/erp-api/src/orders/exception_workbench.rs
  - apps/erp-web/src/pages/orders/review-native/**
disallowed_files:
  - crates/erp-api/src/orders/intake.rs
  - crates/erp-api/src/orders/settlement.rs
shared_lock_targets:
  - crates/erp-api/src/orders.rs
dependencies:
  - 共享锁已登记
dod:
  - OMS 审核主链路回归通过
  - 无越界修改
test_commands:
  - cargo test -p erp-api --test oms_orders -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: commit hash + 测试结果 + 残余风险
---

## 目标
统一订单审核与例外处理动作规则。

## 业务背景
用于降低审核链路回归并提升主链路稳定性。

## DoD
- OMS 审核主链路回归通过
- 无越界修改

## 依赖前提
- 共享锁已登记

## 测试命令
- `cargo test -p erp-api --test oms_orders -- --nocapture`

## 提交格式
- `task | commit | changed_files | test_results | residual_risks`
