> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-02.md。

---
name: ERP95-03 channel integration and sync operations
goal: Close the operations workflow for channel access, shop health, sync jobs, and compensation handling.
background: Industrial ERP delivery needs more than inbound adapters. Operations teams need visibility into authorization state, failed runs, retry paths, and sync health.
owner: Codex-03
branch: codex/erp95-03-channel-ops
worktree: .worktrees/erp95-03
allowed_files:
  - apps/erp-web/src/pages/system/SystemIntegrationsPage.tsx
  - apps/erp-web/src/pages/system/SystemSyncJobsPage.tsx
  - apps/erp-web/src/pages/channels/ChannelDirectoryPage.tsx
  - apps/erp-web/src/pages/channels/ShopAccessPage.tsx
  - apps/erp-web/src/api/systemIntegrations.ts
  - apps/erp-web/src/api/syncJobs.ts
  - crates/erp-repo/src/sync_jobs.rs
  - crates/erp-repo/src/sync_job_runs.rs
  - crates/erp-repo/src/platform_tokens.rs
  - crates/erp-api/tests/platform_oauth_webhook_integration.rs
  - crates/erp-api/tests/system_integrations_integration.rs
  - apps/erp-web/e2e/system-integrations.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Stay compatible with the permission baseline from Track-02.
  - Do not add compensation buttons without log and state evidence.
dod:
  - Frontend shows authorization status, recent runs, failure reasons, and manual compensation entry points.
  - sync_jobs, sync_job_runs, and platform_tokens support the required UI data.
  - system_integrations and OAuth webhook integration tests pass.
  - System integrations E2E passes.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test platform_oauth_webhook_integration --test system_integrations_integration -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/system-integrations.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Close the operations workflow for channel access, shop health, sync jobs, and compensation handling.

## 涓氬姟鑳屾櫙
Industrial ERP delivery needs more than inbound adapters. Operations teams need visibility into authorization state, failed runs, retry paths, and sync health.

## DoD
- Frontend shows authorization status, recent runs, failure reasons, and manual compensation entry points.
- sync_jobs, sync_job_runs, and platform_tokens support the required UI data.
- system_integrations and OAuth webhook integration tests pass.
- System integrations E2E passes.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Stay compatible with the permission baseline from Track-02.
- Do not add compensation buttons without log and state evidence.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test platform_oauth_webhook_integration --test system_integrations_integration -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/system-integrations.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

