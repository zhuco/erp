> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-05.md。

---
name: ERP95-10 integration gates and evidence pack
goal: Integrate tracks 01 through 09 and own task cards, gates, CI, observability, and the final evidence pack.
background: The 95 percent target only matters if one integration track converts all parallel work into one gated, observable, and reviewable release candidate with a final go or no-go decision.
owner: Codex-10
branch: codex/erp95-10-integration-gate
worktree: .worktrees/erp95-10
allowed_files:
  - package.json
  - .github/workflows/ci.yml
  - scripts/release_gate_backend.mjs
  - scripts/verify_openapi_gate_v2.mjs
  - scripts/verify_repo_baseline.mjs
  - docs/ops/RELEASE_GATE_CHECKLIST_V2.md
  - docs/ops/GO_NO_GO_CHECKLIST_V2.md
  - docs/ops/TASK_COMPLETION_MATRIX_V2.md
  - docs/ops/progress_check.md
  - apps/erp-web/src/pages/system/ObservabilityPage.tsx
  - apps/erp-web/src/pages/system/frontendRouteObservability.ts
  - infra/observability/prometheus/alerts.yml
  - docs/spec/erp-launch-full-feature-matrix.md
  - docs/ops/tasks/2026-04-14/erp95-track-01.md
  - docs/ops/tasks/2026-04-14/erp95-track-02.md
  - docs/ops/tasks/2026-04-14/erp95-track-03.md
  - docs/ops/tasks/2026-04-14/erp95-track-04.md
  - docs/ops/tasks/2026-04-14/erp95-track-05.md
  - docs/ops/tasks/2026-04-14/erp95-track-06.md
  - docs/ops/tasks/2026-04-14/erp95-track-07.md
  - docs/ops/tasks/2026-04-14/erp95-track-08.md
  - docs/ops/tasks/2026-04-14/erp95-track-09.md
  - docs/ops/tasks/2026-04-14/erp95-track-10.md
disallowed_files:
  - apps/erp-web/src/pages/orders/**
  - apps/erp-web/src/pages/afterSales/**
  - apps/erp-web/src/pages/procurement/**
  - apps/erp-web/src/pages/planning/**
  - apps/erp-web/src/pages/wms/**
  - apps/erp-web/src/pages/products/**
  - apps/erp-web/src/pages/finance/**
  - crates/erp-repo/src/orders.rs
  - crates/erp-repo/src/wms.rs
  - crates/erp-repo/src/finance.rs
dependencies:
  - Register shared file locks before coding.
  - Generate and maintain all 10 task cards.
  - Run the final full gate in a real database environment or a higher-resource CI runner.
dod:
  - All 10 task cards exist with complete metadata fields.
  - repo:verify, backend release gate, frontend build, and release-gate E2E are part of one integrated gate path.
  - Observability and alerting cover the main chains touched in this program.
  - The feature matrix is updated to the current status.
  - The final go or no-go evidence pack is produced.
  - No out-of-scope edits.
test_commands:
  - pnpm run repo:verify
  - pnpm run test:backend:release-gate
  - pnpm --dir apps/erp-web run generate:api
  - pnpm --dir apps/erp-web build
  - pnpm run test:e2e:release-gate
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Integrate tracks 01 through 09 and own task cards, gates, CI, observability, and the final evidence pack.

## 涓氬姟鑳屾櫙
The 95 percent target only matters if one integration track converts all parallel work into one gated, observable, and reviewable release candidate with a final go or no-go decision.

## DoD
- All 10 task cards exist with complete metadata fields.
- repo:verify, backend release gate, frontend build, and release-gate E2E are part of one integrated gate path.
- Observability and alerting cover the main chains touched in this program.
- The feature matrix is updated to the current status.
- The final go or no-go evidence pack is produced.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Generate and maintain all 10 task cards.
- Run the final full gate in a real database environment or a higher-resource CI runner.

## 娴嬭瘯鍛戒护
- `pnpm run repo:verify`
- `pnpm run test:backend:release-gate`
- `pnpm --dir apps/erp-web run generate:api`
- `pnpm --dir apps/erp-web build`
- `pnpm run test:e2e:release-gate`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

