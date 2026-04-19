---
name: ERP95-5T-02 渠道接入与商品治理
goal: 收口渠道授权、同步作业、失败补偿、商品下载、映射治理、差异分析与变更控制，形成一条可运营可追踪的渠道商品闭环。
background: 本任务卡吸收原 ERP95-03 与 ERP95-04 的内容，把“渠道运营工作台”和“商品治理工作台”合并，避免状态口径和共享数据模型割裂。
owner: Codex-5T-02
branch: codex/erp95-5t-02-channel-product
worktree: .worktrees/erp95-03
action: active
source_plan: docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md
replaces:
  - docs/ops/tasks/2026-04-14/erp95-track-03.md
  - docs/ops/tasks/2026-04-14/erp95-track-04.md
allowed_files:
  - apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx
  - apps/erp-web/src/pages/system/SystemSyncJobsPage.tsx
  - apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx
  - apps/erp-web/src/api/systemIntegrations.ts
  - apps/erp-web/src/api/syncJobs.ts
  - apps/erp-web/src/pages/products/ProductsPage.tsx
  - apps/erp-web/src/pages/products/ProductChangeControlPage.tsx
  - apps/erp-web/src/pages/products/ProductChannelAvailabilityPage.tsx
  - apps/erp-web/src/pages/products/ProductDiffPage.tsx
  - apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx
  - apps/erp-web/src/api/products.ts
  - apps/erp-web/src/api/shopProducts.ts
  - crates/erp-repo/src/sync_jobs.rs
  - crates/erp-repo/src/sync_job_runs.rs
  - crates/erp-repo/src/platform_tokens.rs
  - crates/erp-repo/src/products.rs
  - crates/erp-repo/src/shop_products.rs
  - crates/erp-repo/src/channel_sku_mappings.rs
  - crates/erp-repo/src/shop_product_tasks.rs
  - crates/erp-api/tests/platform_oauth_webhook_integration.rs
  - crates/erp-api/tests/system_integrations_integration.rs
  - crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs
  - apps/erp-web/e2e/system-integrations.spec.ts
  - apps/erp-web/e2e/shop-product-match-governance.spec.ts
  - apps/erp-web/e2e/product-change-control.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/**
  - apps/erp-web/src/pages/channels/ShopAccessPage.tsx
  - apps/erp-web/src/pages/orders/**
  - apps/erp-web/src/pages/wms/**
  - apps/erp-web/src/pages/procurement/**
  - apps/erp-web/src/pages/planning/**
  - apps/erp-web/src/pages/afterSales/**
  - apps/erp-web/src/pages/finance/**
  - package.json
  - scripts/**
  - docs/ops/**
dependencies:
  - 兼容 Track-01 产出的权限基线和 ShopAccessPage 接口结果。
  - 优先吸收 erp95-03、erp95-04 中已经成形的页面与测试进度。
  - 不为了追求自动映射覆盖率而牺牲手工治理与冲突可追踪性。
dod:
  - 授权状态、同步运行、失败原因、补偿入口可见。
  - ProductDiffPage 不再是占位页。
  - 商品下载、映射结果、冲突原因、回写结果可追踪。
  - 渠道与商品治理相关后端测试和 E2E 具备通过路径。
  - 无越界修改。
test_commands:
  - cargo test -p erp-api --test platform_oauth_webhook_integration --test system_integrations_integration --test platform_sync_and_mapping_flow_integration -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/system-integrations.spec.ts e2e/shop-product-match-governance.spec.ts e2e/product-change-control.spec.ts --project=chromium
delivery: commit hash + 变更摘要 + 测试结果 + 剩余风险
---

## 任务目标
把渠道接入和商品治理从两段式工作流改成一条连续工作流，确保运营人员可以从“授权是否正常”一路看到“下载了什么、映射到哪里、为什么冲突、如何修复”。

## 执行步骤
- [ ] Step 1: 盘点 `erp95-03` 与 `erp95-04` 的已有可复用成果。
- [ ] Step 2: 先跑失败测试，锁定授权、同步、映射、差异分析的缺口。
- [ ] Step 3: 完成渠道运营工作台所需的状态与补偿信息。
- [ ] Step 4: 完成商品下载、差异分析、映射治理和变更控制联动。
- [ ] Step 5: 统一渠道 token、同步任务、店铺商品、主商品映射的状态口径。
- [ ] Step 6: 跑通后端测试与前端 E2E，并记录剩余自动化不足项。

## 特别注意
- `ShopAccessPage.tsx` 不在本卡片白名单内，禁止争抢。
- 本卡片优先保证可运营、可解释、可修复，不强行扩展成复杂策略引擎。
- 若自动映射不稳，允许退回人工治理，但必须保留冲突原因与回写证据。
