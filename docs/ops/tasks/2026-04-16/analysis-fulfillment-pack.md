---
name: 洞察履约分析包
goal: 落地履约时效与订单履约周期两类 Phase 1 页面，并为履约台账类功能建立高质量占位页
background: 对应洞察域中的履约与仓配效率分组，优先复用 shipping 与 sales-cycle 合同
owner: worker-fulfillment-01
branch: codex/analysis-fulfillment-pack-20260416
worktree: .worktrees/analysis-fulfillment-pack-20260416
allowed_files:
  - apps/erp-web/src/pages/insights/fulfillment/**
  - apps/erp-web/src/pages/stats/ShippingStatsPage.tsx
  - apps/erp-web/src/pages/stats/SalesCyclePage.tsx
  - apps/erp-web/src/pages/stats/StatsPlaceholder.tsx
  - docs/spec/18_insights_function_details.md
disallowed_files:
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/navigation/routeAliases.ts
  - apps/erp-web/src/api/stats.ts
dependencies:
  - 共享文件锁已由 integrator-01 持有 履约页面只负责领域实现
  - 必须覆盖 docs/spec/18_insights_function_details.md 中履约与仓配效率章节
dod:
  - 履约时效与订单履约周期页面已可渲染
  - 收发作业流水 包装耗用 盘点偏差 作业准确度 仓网效能占位页已具备口径说明
  - 不修改共享路由与菜单文件
  - 无越界修改
test_commands:
  - pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts
retry_policy: transient-auto-retry(max=2,delay=8s)
fallback_strategy: switch-to-local-execution
delivery: commit hash + 页面清单 + 测试结果 + 待新增主题接口列表
---

## Goal
落地履约时效与订单履约周期两类 Phase 1 页面，并为履约台账类功能建立高质量占位页。

## Background
对应洞察域中的履约与仓配效率分组，优先复用现有 `shipping` 与 `sales-cycle` 合同。

## DoD
- 履约时效与订单履约周期页面已可渲染
- 收发作业流水、包装耗用、盘点偏差、作业准确度、仓网效能占位页已具备口径说明
- 不修改共享路由与菜单文件
- 无越界修改

## Dependencies
- 共享文件锁已由 `integrator-01` 持有，履约页面只负责领域实现
- 必须覆盖 `docs/spec/18_insights_function_details.md` 中履约与仓配效率章节

## Test Commands
- `pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts`

## Retry Policy
transient-auto-retry(max=2,delay=8s)

## Fallback Strategy
switch-to-local-execution

## Delivery
commit hash + 页面清单 + 测试结果 + 待新增主题接口列表
