---
name: 售后 SLA 收敛示例
goal: 修复售后超时率统计并补齐页面展示
background: 用于售后工业级收敛与客服协同
owner: integrator-01
branch: codex/aftersales-sla-20260414
worktree: .worktrees/aftersales-sla-20260414
allowed_files:
  - crates/erp-api/tests/after_sales_sla_escalation.rs
  - crates/erp-repo/src/after_sales_sla_rules.rs
  - apps/erp-web/src/pages/afterSales/AfterSalesSlaPage.tsx
disallowed_files:
  - crates/erp-repo/src/audit_logs.rs
dependencies:
  - 共享文件锁已登记
dod:
  - 通过 after_sales_sla_escalation 测试
  - 无越界修改
test_commands:
  - cargo test -p erp-api --test after_sales_sla_escalation -- --nocapture
delivery: Markdown + 相关资源
---

## 目标
修复售后超时率统计并补齐页面展示

## 业务背景
用于售后工业级收敛与客服协同

## DoD
- 通过 after_sales_sla_escalation 测试
- 无越界修改

## 依赖前提
- 共享文件锁已登记

## 测试命令
- `cargo test -p erp-api --test after_sales_sla_escalation -- --nocapture`

## 交付格式
Markdown + 相关资源
