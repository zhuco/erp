---
name: W9-T1 渠道店铺映射规则与冲突检测
goal: 完善映射规则与冲突检测，降低错配风险。
background: 渠道映射错误会直接影响同步、履约与库存联动。
owner: W9-owner
window: W9
domain: channels-mapping-sync
slot: T1
branch: codex/w9-channels-mapping-sync-t1-20260414
worktree: .worktrees/w9-channels-mapping-sync-t1-20260414
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
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w9-t1-channel-shop-mapping-rules.md --mode all
  - cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
完善映射规则与冲突检测，降低错配风险。

## 业务背景
渠道映射错误会直接影响同步、履约与库存联动。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w9-t1-channel-shop-mapping-rules.md --mode all`
- `cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
