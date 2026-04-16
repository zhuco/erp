---
name: 洞察售后与工坊分析包
goal: 承接报表中心与导出队列的新洞察入口，并为售后治理类分析页建立完整规格占位
background: 对应售后与补偿治理、报表与导出工坊两大分组，优先复用 reports 与 exports 页面
owner: worker-service-workbench-01
branch: codex/analysis-service-workbench-pack-20260416
worktree: .worktrees/analysis-service-workbench-pack-20260416
allowed_files:
  - apps/erp-web/src/pages/insights/service/**
  - apps/erp-web/src/pages/insights/workbench/**
  - apps/erp-web/src/pages/reports/ReportsPage.tsx
  - apps/erp-web/src/pages/exports/ExportJobsPage.tsx
  - docs/spec/18_insights_function_details.md
disallowed_files:
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/navigation/routeAliases.ts
  - apps/erp-web/src/api/reports.ts
dependencies:
  - 共享文件锁已由 integrator-01 持有 路由与菜单由主控统一接入
  - 必须按 docs/spec/18_insights_function_details.md 中售后与工坊章节实现
dod:
  - 报表中心与导出队列已具备洞察域 canonical 页面组件
  - 退回闭环 补偿看板 补发质量 工单效率已有高质量占位页
  - 无越界修改
test_commands:
  - pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts
retry_policy: transient-auto-retry(max=2,delay=8s)
fallback_strategy: switch-to-local-execution
delivery: commit hash + 页面清单 + 测试结果 + 待接入的售后主题合同
---

## Goal
承接报表中心与导出队列的新洞察入口，并为售后治理类分析页建立完整规格占位。

## Background
对应售后与补偿治理、报表与导出工坊两大分组，优先复用 `reports` 与 `exports` 页面。

## DoD
- 报表中心与导出队列已具备洞察域 canonical 页面组件
- 退回闭环、补偿看板、补发质量、工单效率已有高质量占位页
- 无越界修改

## Dependencies
- 共享文件锁已由 `integrator-01` 持有，路由与菜单由主控统一接入
- 必须按 `docs/spec/18_insights_function_details.md` 中售后与工坊章节实现

## Test Commands
- `pnpm --dir apps/erp-web playwright test e2e/task8-stats-observability-sso.spec.ts`

## Retry Policy
transient-auto-retry(max=2,delay=8s)

## Fallback Strategy
switch-to-local-execution

## Delivery
commit hash + 页面清单 + 测试结果 + 待接入的售后主题合同
