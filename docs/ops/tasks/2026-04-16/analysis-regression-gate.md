---
name: 洞察路由回归与发布门禁
goal: 统一注册 Phase 1 洞察路由 补齐 legacy redirect 并完成导航与统计回归门禁
background: 用于所有洞察子任务合流后的收口验证，保证新旧地址 菜单导航和健康信息可用
owner: integrator-gate-01
branch: codex/analysis-regression-gate-20260416
worktree: .worktrees/analysis-regression-gate-20260416
allowed_files:
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/navigation/routeAliases.ts
  - apps/erp-web/src/app/layout/routeMeta.ts
  - apps/erp-web/e2e/workbench-menu-navigation.spec.ts
  - apps/erp-web/e2e/task8-stats-observability-sso.spec.ts
  - docs/ops/tasks/2026-04-16/**
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/api/stats.ts
  - apps/erp-web/src/api/reports.ts
dependencies:
  - 必须在四个领域包完成并给出 route manifest 后开始
  - 共享文件锁回收后方可改 router.tsx
dod:
  - Phase 1 洞察 canonical route 全部可访问
  - legacy /stats/* /reports /exports 均已重定向
  - 菜单导航与统计健康回归用例已覆盖
  - 无越界修改
test_commands:
  - pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.ts
  - pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts e2e/task8-stats-observability-sso.spec.ts
retry_policy: transient-auto-retry(max=2,delay=8s)
fallback_strategy: switch-to-local-execution
delivery: commit hash + 回归结果 + redirect 清单 + 残余风险
---

## Goal
统一注册 Phase 1 洞察路由、补齐 legacy redirect，并完成导航与统计回归门禁。

## Background
用于所有洞察子任务合流后的收口验证，保证新旧地址、菜单导航和健康信息可用。

## DoD
- Phase 1 洞察 canonical route 全部可访问
- legacy `/stats/*`、`/reports`、`/exports` 均已重定向
- 菜单导航与统计健康回归用例已覆盖
- 无越界修改

## Dependencies
- 必须在四个领域包完成并给出 route manifest 后开始
- 共享文件锁回收后方可改 `router.tsx`

## Test Commands
- `pnpm --dir apps/erp-web vitest run src/app/layout/headerMenuModel.test.ts src/app/layout/headerSharedSubmenu.test.ts`
- `pnpm --dir apps/erp-web playwright test e2e/workbench-menu-navigation.spec.ts e2e/task8-stats-observability-sso.spec.ts`

## Retry Policy
transient-auto-retry(max=2,delay=8s)

## Fallback Strategy
switch-to-local-execution

## Delivery
commit hash + 回归结果 + redirect 清单 + 残余风险
