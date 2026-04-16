---
name: 洞察壳层与共享文件锁
goal: 建立洞察域基础信息架构与共享文件锁，统一菜单、标题和 route alias，避免并行任务互踩
background: 本任务是分析菜单工业级重构第一阶段的主控收口任务，只负责共享文件和 IA 壳层，不负责具体领域页实现
owner: integrator-01
branch: codex/analysis-insights-shell-20260416
worktree: .worktrees/analysis-insights-shell-20260416
allowed_files:
  - apps/erp-web/src/features/insights/insightFeatureCatalog.ts
  - apps/erp-web/src/features/insights/insightDimensionCatalog.ts
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/layout/headerMenuModel.ts
  - apps/erp-web/src/app/navigation/routeAliases.ts
  - apps/erp-web/src/app/layout/routeMeta.ts
  - docs/superpowers/specs/2026-04-16-analysis-menu-industrial-refactor-design.md
  - docs/superpowers/plans/2026-04-16-analysis-menu-industrial-refactor-implementation.md
disallowed_files:
  - apps/erp-web/src/pages/insights/**
  - apps/erp-web/src/pages/stats/**
  - apps/erp-web/src/pages/reports/ReportsPage.tsx
  - apps/erp-web/src/pages/exports/ExportJobsPage.tsx
dependencies:
  - 必须先阅读 docs/superpowers/specs/2026-04-16-analysis-menu-industrial-refactor-design.md
  - 该任务持有共享文件锁 menu.tsx routeAliases.ts routeMeta.ts headerMenuModel.ts
dod:
  - 洞察域 feature catalog 与 dimension catalog 已落库
  - 顶部模块已从分析升级为洞察且分组与设计文档一致
  - legacy route 映射清单已定义
  - 无越界修改
test_commands:
  - pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.ts
retry_policy: transient-auto-retry(max=2,delay=8s)
fallback_strategy: switch-to-local-execution
delivery: commit hash + 共享文件锁说明 + 测试结果 + 残余风险
---

## Goal
建立洞察域基础信息架构与共享文件锁，统一菜单、标题和 route alias，避免并行任务互踩。

## Background
本任务是分析菜单工业级重构第一阶段的主控收口任务，只负责共享文件和 IA 壳层，不负责具体领域页实现。

## DoD
- 洞察域 feature catalog 与 dimension catalog 已落库
- 顶部模块已从分析升级为洞察且分组与设计文档一致
- legacy route 映射清单已定义
- 无越界修改

## Dependencies
- 必须先阅读 `docs/superpowers/specs/2026-04-16-analysis-menu-industrial-refactor-design.md`
- 该任务持有共享文件锁 `menu.tsx` `routeAliases.ts` `routeMeta.ts` `headerMenuModel.ts`

## Test Commands
- `pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.ts`

## Retry Policy
transient-auto-retry(max=2,delay=8s)

## Fallback Strategy
switch-to-local-execution

## Delivery
commit hash + 共享文件锁说明 + 测试结果 + 残余风险
