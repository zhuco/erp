---
name: ERP95-5T-01 平台与生产入口基线
goal: 收口生产菜单、路由、模块搜索与最小租户组织权限基线，建立首发版本可用的生产入口和平台隔离能力。
background: 本任务卡吸收原 ERP95-01 与 ERP95-02 的内容，不再按“菜单治理”和“多商户基线”分开推进，避免共享文件冲突与责任分散。
owner: Codex-5T-01
branch: codex/erp95-5t-01-platform-ia
worktree: .worktrees/erp95-01
action: active
source_plan: docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-five-track-codex-plan.md
replaces:
  - docs/ops/tasks/2026-04-14/erp95-track-01.md
  - docs/ops/tasks/2026-04-14/erp95-track-02.md
allowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/layout/routeMeta.ts
  - apps/erp-web/src/app/layout/moduleSearch.ts
  - apps/erp-web/src/app/layout/moduleSearch.test.ts
  - apps/erp-web/src/pages/sales/SalesEcommercePlaceholderPages.tsx
  - apps/erp-web/src/pages/system/AgGridEnterpriseDemoPage.tsx
  - apps/erp-web/src/pages/system/DataScopesPage.tsx
  - apps/erp-web/src/pages/system/UsersPage.tsx
  - apps/erp-web/src/pages/channels/ShopAccessPage.tsx
  - crates/erp-repo/src/tenants.rs
  - crates/erp-repo/src/data_scopes.rs
  - crates/erp-repo/src/permissions.rs
  - crates/erp-api/src/lib.rs
  - crates/erp-api/tests/api_rbac_tenant_isolation.rs
  - migrations/0095_multi_merchant_org_baseline.sql
  - apps/erp-web/e2e/workbench-menu-navigation.spec.ts
  - apps/erp-web/e2e/frontend-route-observability.spec.ts
  - apps/erp-web/e2e/channels-shops-list-visibility.spec.ts
disallowed_files:
  - apps/erp-web/src/pages/products/**
  - apps/erp-web/src/pages/orders/**
  - apps/erp-web/src/pages/wms/**
  - apps/erp-web/src/pages/procurement/**
  - apps/erp-web/src/pages/planning/**
  - apps/erp-web/src/pages/afterSales/**
  - apps/erp-web/src/pages/finance/**
  - package.json
  - scripts/**
  - docs/ops/**
  - .github/workflows/**
dependencies:
  - 先吸收或清理 erp95-01、erp95-02 中的已有改动，再继续编码。
  - 如果 erp95-02 中存在越界文件，必须先做边界消毒。
  - ShopAccessPage.tsx 归本卡片所有，其他新卡不得争抢。
dod:
  - 生产菜单不再暴露演示页或建设中主入口。
  - 菜单、路由、模块搜索保持一致。
  - 最小 merchant/org/shop 权限基线可验证。
  - api_rbac_tenant_isolation 与导航相关测试具备通过路径。
  - 无越界修改。
test_commands:
  - cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture
  - pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts
  - pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/frontend-route-observability.spec.ts e2e/channels-shops-list-visibility.spec.ts --project=chromium
delivery: commit hash + 变更摘要 + 测试结果 + 剩余风险
---

## 任务目标
把“生产入口治理”和“平台最小隔离基线”收敛成一个可交付任务，先解决项目最容易造成误判和串线的入口问题，再补齐首发所需的租户/组织/店铺级最小权限能力。

## 执行步骤
- [ ] Step 1: 盘点 `erp95-01` 与 `erp95-02` 的现有改动，识别需要吸收与需要剥离的内容。
- [ ] Step 2: 先运行失败测试，锁定路由暴露和租户隔离缺口。
- [ ] Step 3: 完成菜单、路由、模块搜索与占位页清理。
- [ ] Step 4: 完成最小 merchant/org/shop 数据权限与展示基线。
- [ ] Step 5: 跑通前端入口与权限隔离验证。
- [ ] Step 6: 输出平台基线假设与剩余边界说明。

## 特别注意
- 本卡片不扩展多法人、总账、复杂结算主体。
- 发现越界后先修边界，不允许带着污染继续推进。
- 本卡片完成后，其他卡片默认以这里的入口与权限基线为准。
