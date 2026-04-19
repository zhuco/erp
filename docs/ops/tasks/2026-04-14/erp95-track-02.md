> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-01.md。

---
name: ERP95-02 multi-merchant and org baseline
goal: Add the minimum multi-merchant, multi-organization, and shop-level permission baseline required for the 95 percent target.
background: The current project still has a structural P0 gap around the multi-merchant operating model. This track closes the smallest shippable baseline across schema, repo, API, and management pages.
owner: Codex-02
branch: codex/erp95-02-merchant-org
worktree: .worktrees/erp95-02
allowed_files:
  - migrations/0095_multi_merchant_org_baseline.sql
  - crates/erp-repo/src/tenants.rs
  - crates/erp-repo/src/data_scopes.rs
  - crates/erp-repo/src/permissions.rs
  - crates/erp-api/src/lib.rs
  - crates/erp-api/tests/api_rbac_tenant_isolation.rs
  - apps/erp-web/src/pages/system/DataScopesPage.tsx
  - apps/erp-web/src/pages/system/UsersPage.tsx
  - apps/erp-web/src/pages/channels/ShopAccessPage.tsx
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - package.json
  - scripts/**
  - docs/ops/**
dependencies:
  - Register shared file locks before coding.
  - Stay inside the Track-02 file boundary.
  - Keep the migration number conflict-free with the current branch head.
dod:
  - A minimum merchant and org baseline migration exists and compiles.
  - API permission checks block cross-entity access.
  - DataScopesPage, UsersPage, and ShopAccessPage expose the minimum management baseline.
  - api_rbac_tenant_isolation passes.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/channels-shops-list-visibility.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Add the minimum multi-merchant, multi-organization, and shop-level permission baseline required for the 95 percent target.

## 涓氬姟鑳屾櫙
The current project still has a structural P0 gap around the multi-merchant operating model. This track closes the smallest shippable baseline across schema, repo, API, and management pages.

## DoD
- A minimum merchant and org baseline migration exists and compiles.
- API permission checks block cross-entity access.
- DataScopesPage, UsersPage, and ShopAccessPage expose the minimum management baseline.
- api_rbac_tenant_isolation passes.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Stay inside the Track-02 file boundary.
- Keep the migration number conflict-free with the current branch head.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test api_rbac_tenant_isolation -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/channels-shops-list-visibility.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

