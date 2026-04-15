---
name: W8-T3 商品与 SKU 链路回归收口
goal: 输出商品/SKU 窗口批次回归证据。
background: T3 负责主数据窗口合并前收口，避免主干回归。
owner: W8-owner
window: W8
domain: product-sku-masterdata
slot: T3
branch: codex/w8-product-sku-masterdata-t3-20260414
worktree: .worktrees/w8-product-sku-masterdata-t3-20260414
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
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w8-t3-product-sku-regression-pack.md --mode all
  - cargo test -p erp-api --test skus -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/products.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
输出商品/SKU 窗口批次回归证据。

## 业务背景
T3 负责主数据窗口合并前收口，避免主干回归。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w8-t3-product-sku-regression-pack.md --mode all`
- `cargo test -p erp-api --test skus -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/products.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
