---
name: W9-T3 渠道映射同步回归收口
goal: 完成渠道窗口批次回归并输出残余风险。
background: T3 汇总渠道域证据，为主控提供合并判断依据。
owner: W9-owner
window: W9
domain: channels-mapping-sync
slot: T3
branch: codex/w9-channels-mapping-sync-t3-20260414
worktree: .worktrees/w9-channels-mapping-sync-t3-20260414
allowed_files:
  - crates/erp-repo/src/channels.rs
  - crates/erp-repo/src/shops.rs
  - crates/erp-repo/src/mapping*.rs
  - crates/erp-repo/src/shop_products.rs
  - crates/adapter-*/**
  - apps/erp-web/src/pages/channels/**
disallowed_files:
  - openapi/src/**
  - openapi/erp_api_v1.yaml
shared_lock_targets: []
dependencies:
  - 任务卡已创建并确认窗口边界
  - T1/T2 已有候选提交
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w9-t3-channel-mapping-regression-pack.md --mode all
  - cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/shop-product-match-governance.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完成渠道窗口批次回归并输出残余风险。

## 业务背景
T3 汇总渠道域证据，为主控提供合并判断依据。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1/T2 已有候选提交

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w9-t3-channel-mapping-regression-pack.md --mode all`
- `cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/shop-product-match-governance.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
