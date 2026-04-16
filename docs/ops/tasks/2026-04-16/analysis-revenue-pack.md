---
name: 洞察收入分析包
goal: 落地交易总览 营运日报 增长雷达三类 Phase 1 页面，并为订单结构和让利诊断建立可交付占位页
background: 对应洞察域中的收入与订单质量分组，优先复用现有 sales sales-daily sales-trends 合同
owner: worker-revenue-01
branch: codex/analysis-revenue-pack-20260416
worktree: .worktrees/analysis-revenue-pack-20260416
allowed_files:
  - apps/erp-web/src/pages/insights/revenue/**
  - apps/erp-web/src/pages/stats/SalesStatsPage.tsx
  - apps/erp-web/src/pages/stats/SalesDailyReportPage.tsx
  - apps/erp-web/src/pages/stats/SalesTrendAnalysisPage.tsx
  - apps/erp-web/src/pages/stats/StatsPlaceholder.tsx
  - docs/spec/18_insights_function_details.md
disallowed_files:
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/navigation/routeAliases.ts
  - apps/erp-web/src/api/stats.ts
dependencies:
  - 共享文件锁已由 integrator-01 持有 路由注册不能在本任务内完成
  - 必须按 docs/spec/18_insights_function_details.md 中的收入类规格实现
dod:
  - 交易总览 营运日报 增长雷达已有可访问页面组件
  - 订单结构 让利诊断具备明确的占位页和口径说明
  - 不修改共享路由与菜单文件
  - 无越界修改
test_commands:
  - pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts
retry_policy: transient-auto-retry(max=2,delay=8s)
fallback_strategy: switch-to-local-execution
delivery: commit hash + 页面清单 + 测试结果 + 仍缺失的后端合同
---

## Goal
落地交易总览、营运日报、增长雷达三类 Phase 1 页面，并为订单结构和让利诊断建立可交付占位页。

## Background
对应洞察域中的收入与订单质量分组，优先复用现有 `sales`、`sales-daily`、`sales-trends` 合同。

## DoD
- 交易总览、营运日报、增长雷达已有可访问页面组件
- 订单结构、让利诊断具备明确的占位页和口径说明
- 不修改共享路由与菜单文件
- 无越界修改

## Dependencies
- 共享文件锁已由 `integrator-01` 持有，路由注册不能在本任务内完成
- 必须按 `docs/spec/18_insights_function_details.md` 中的收入类规格实现

## Test Commands
- `pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts`

## Retry Policy
transient-auto-retry(max=2,delay=8s)

## Fallback Strategy
switch-to-local-execution

## Delivery
commit hash + 页面清单 + 测试结果 + 仍缺失的后端合同
