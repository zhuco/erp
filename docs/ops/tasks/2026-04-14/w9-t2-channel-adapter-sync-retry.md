---
name: W9-T2 渠道同步重试与错误分类
goal: 增强同步失败重试与错误分类能力，提升稳定性。
background: 多平台适配器失败场景复杂，需要统一重试策略。
owner: W9-owner
window: W9
domain: channels-mapping-sync
slot: T2
branch: codex/w9-channels-mapping-sync-t2-20260414
worktree: .worktrees/w9-channels-mapping-sync-t2-20260414
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
  - T1 核心字段契约已稳定
dod:
  - 边界校验 PASS，且无越界改动
  - 任务卡规定的最小测试通过
  - 按标准回执格式输出结果与残余风险
test_commands:
  - node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w9-t2-channel-adapter-sync-retry.md --mode all
  - cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture
  - pnpm --filter erp-web test:e2e -- e2e/channels-shops-list-visibility.spec.ts
submission_format: task | commit | changed_files | test_results | residual_risks
delivery: 按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
---

## 目标
增强同步失败重试与错误分类能力，提升稳定性。

## 业务背景
多平台适配器失败场景复杂，需要统一重试策略。

## 完成定义（DoD）
- 边界校验 PASS，且无越界改动
- 任务卡规定的最小测试通过
- 按标准回执格式输出结果与残余风险

## 前置依赖
- 任务卡已创建并确认窗口边界
- T1 核心字段契约已稳定

## 测试命令
- `node scripts/ops/validate_task_boundary.mjs --task docs/ops/tasks/2026-04-14/w9-t2-channel-adapter-sync-retry.md --mode all`
- `cargo test -p erp-api --test platform_oauth_webhook_integration -- --nocapture`
- `pnpm --filter erp-web test:e2e -- e2e/channels-shops-list-visibility.spec.ts`

## 回执格式
task | commit | changed_files | test_results | residual_risks

## 交付要求
按标准回执格式返回：task / commit / changed_files / test_results / residual_risks
