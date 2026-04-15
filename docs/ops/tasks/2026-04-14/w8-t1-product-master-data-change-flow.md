---
name: W8-T1 商品主数据变更流程加固
goal: 完善商品主数据变更流程与审计字段，提升可追溯性。
background: 主数据质量直接影响订单、库存与渠道一致性。
owner: W8-owner
window: W8
domain: product-sku-masterdata
slot: T1
branch: codex/w8-product-sku-masterdata-t1-20260414
worktree: .worktrees/w8-product-sku-masterdata-t1-20260414
allowed_files:
  - crates/erp-repo/src/product*.rs
  - crates/erp-repo/src/sku*.rs
  - apps/erp-web/src/pages/products/**
  - apps/erp-web/src/pages/skus/**
disallowed_files:
  - crates/erp-repo/src/mapping*.rs
  - crates/erp-repo/src/shop_products.rs
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w8-t1-product-master-data-change-flow.md --mode all
  - cargo test -p erp-api --test skus -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善商品主数据变更流程与审计字段，提升可追溯性。

## 业务背景
主数据质量直接影响订单、库存与渠道一致性。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w8-t1-product-master-data-change-flow.md --mode all`
- `cargo test -p erp-api --test skus -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
