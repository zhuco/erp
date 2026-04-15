> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-01.md。

---
name: ERP95-01 IA and production navigation cleanup
goal: Clean up production menu, route reachability, demo entry isolation, and sales placeholder entry exposure.
background: This track closes the information architecture gap for the first commercial release so production navigation only exposes real workflows instead of demo pages or build-in-progress entries.
owner: Codex-01
branch: codex/erp95-01-ia
worktree: .worktrees/erp95-01
allowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - apps/erp-web/src/app/layout/routeMeta.ts
  - apps/erp-web/src/app/layout/moduleSearch.ts
  - apps/erp-web/src/app/layout/moduleSearch.test.ts
  - apps/erp-web/src/pages/sales/SalesEcommercePlaceholderPages.tsx
  - apps/erp-web/src/pages/system/AgGridEnterpriseDemoPage.tsx
  - apps/erp-web/e2e/workbench-menu-navigation.spec.ts
  - apps/erp-web/e2e/frontend-route-observability.spec.ts
disallowed_files:
  - package.json
  - scripts/**
  - docs/ops/**
  - .github/workflows/**
dependencies:
  - Use docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-ten-track-codex-plan.md as the source plan.
  - Register shared file locks before coding.
  - Do not change business pages outside the navigation cleanup scope.
dod:
  - Production menu no longer exposes AgGridEnterpriseDemoPage as a release entry.
  - Sales production entry no longer lands on a build-in-progress placeholder page.
  - Menu, router, and module search stay consistent.
  - Specified Vitest and Playwright checks pass.
  - No out-of-scope edits.
test_commands:
  - pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts
  - pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/frontend-route-observability.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Clean up production menu, route reachability, demo entry isolation, and sales placeholder entry exposure.

## 涓氬姟鑳屾櫙
This track closes the information architecture gap for the first commercial release so production navigation only exposes real workflows instead of demo pages or build-in-progress entries.

## DoD
- Production menu no longer exposes AgGridEnterpriseDemoPage as a release entry.
- Sales production entry no longer lands on a build-in-progress placeholder page.
- Menu, router, and module search stay consistent.
- Specified Vitest and Playwright checks pass.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Use docs/superpowers/plans/2026-04-14-industrial-erp-95-plus-ten-track-codex-plan.md as the source plan.
- Register shared file locks before coding.
- Do not change business pages outside the navigation cleanup scope.

## 娴嬭瘯鍛戒护
- `pnpm --filter erp-web exec vitest --run src/app/layout/moduleSearch.test.ts`
- `pnpm --filter erp-web exec playwright test e2e/workbench-menu-navigation.spec.ts e2e/frontend-route-observability.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

