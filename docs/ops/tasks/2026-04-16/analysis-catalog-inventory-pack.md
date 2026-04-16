---
name: 洞察商品与库存分析包
goal: 落地爆品热度 长尾沉积，并把库存主题页拆成可演进的独立洞察页面
background: 对应商品结构与生命周期、库存健康与调拨两大分组，优先复用 best-sellers slow-moving reports 现有能力
owner: worker-catalog-inventory-01
branch: codex/analysis-catalog-inventory-pack-20260416
worktree: .worktrees/analysis-catalog-inventory-pack-20260416
allowed_files:
  - apps/erp-web/src/pages/insights/catalog/**
  - apps/erp-web/src/pages/insights/inventory/**
  - apps/erp-web/src/pages/stats/BestSellerAnalysisPage.tsx
  - apps/erp-web/src/pages/stats/SlowMovingAnalysisPage.tsx
  - apps/erp-web/src/pages/reports/ReportsPage.tsx
  - docs/spec/18_insights_function_details.md
disallowed_files:
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/navigation/routeAliases.ts
  - apps/erp-web/src/api/stats.ts
  - apps/erp-web/src/api/reports.ts
dependencies:
  - 共享文件锁已由 integrator-01 持有 允许产出 route manifest 但不允许直接改 router
  - 必须按 docs/spec/18_insights_function_details.md 中商品与库存章节实现
dod:
  - 爆品热度与长尾沉积页面具备 canonical 页面组件
  - 收发存总览 库龄健康 周转流速已具备可演进页面
  - 连带组合 调拨效益 负库存预警有明确占位页
  - 无越界修改
test_commands:
  - pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts
retry_policy: transient-auto-retry(max=2,delay=8s)
fallback_strategy: switch-to-local-execution
delivery: commit hash + 页面清单 + 测试结果 + Phase 1.5 2 合同缺口
---

## Goal
落地爆品热度、长尾沉积，并把库存主题页拆成可演进的独立洞察页面。

## Background
对应商品结构与生命周期、库存健康与调拨两大分组，优先复用 `best-sellers`、`slow-moving`、`reports` 现有能力。

## DoD
- 爆品热度与长尾沉积页面具备 canonical 页面组件
- 收发存总览、库龄健康、周转流速已具备可演进页面
- 连带组合、调拨效益、负库存预警有明确占位页
- 无越界修改

## Dependencies
- 共享文件锁已由 `integrator-01` 持有，允许产出 route manifest 但不允许直接改 `router.tsx`
- 必须按 `docs/spec/18_insights_function_details.md` 中商品与库存章节实现

## Test Commands
- `pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts`

## Retry Policy
transient-auto-retry(max=2,delay=8s)

## Fallback Strategy
switch-to-local-execution

## Delivery
commit hash + 页面清单 + 测试结果 + Phase 1.5 与 Phase 2 合同缺口
