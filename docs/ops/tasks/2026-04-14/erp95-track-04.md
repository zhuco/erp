> 本卡片已被 5-track 主计划取代，请勿继续作为当前执行主卡。现行卡片：docs/ops/tasks/2026-04-14/erp95-5track-02.md。

---
name: ERP95-04 product master data and mapping governance
goal: Make product master data, channel product download, mapping governance, diff analysis, and change control executable end to end.
background: Product and mapping capabilities are broad but still too shallow in several areas. This track converts those areas from partial coverage into real operational workflows.
owner: Codex-04
branch: codex/erp95-04-product-governance
worktree: .worktrees/erp95-04
allowed_files:
  - apps/erp-web/src/pages/products/ProductsPage.tsx
  - apps/erp-web/src/pages/products/ProductChangeControlPage.tsx
  - apps/erp-web/src/pages/products/ProductChannelAvailabilityPage.tsx
  - apps/erp-web/src/pages/products/ProductDiffPage.tsx
  - apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx
  - apps/erp-web/src/api/products.ts
  - apps/erp-web/src/api/shopProducts.ts
  - crates/erp-repo/src/products.rs
  - crates/erp-repo/src/shop_products.rs
  - crates/erp-repo/src/channel_sku_mappings.rs
  - crates/erp-repo/src/shop_product_tasks.rs
  - crates/erp-api/tests/platform_sync_and_mapping_flow_integration.rs
  - apps/erp-web/e2e/shop-product-match-governance.spec.ts
  - apps/erp-web/e2e/product-change-control.spec.ts
disallowed_files:
  - apps/erp-web/src/app/layout/menu.tsx
  - apps/erp-web/src/app/router.tsx
  - scripts/**
  - docs/ops/**
  - package.json
dependencies:
  - Register shared file locks before coding.
  - Stay inside the Track-04 write boundary even when sharing data with channel integration work.
  - Evolve ProductDiffPage into a usable diff workflow instead of redesigning the entire product domain.
dod:
  - ProductDiffPage is no longer an empty placeholder shell.
  - Download, mapping task, conflict reason, and write-back flows are traceable.
  - platform_sync_and_mapping_flow_integration passes.
  - Product governance and mapping governance E2E tests pass.
  - No out-of-scope edits.
test_commands:
  - cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
  - pnpm --filter erp-web exec playwright test e2e/shop-product-match-governance.spec.ts e2e/product-change-control.spec.ts --project=chromium
delivery: commit hash + change summary + test results + residual risks
---

## 鐩爣
Make product master data, channel product download, mapping governance, diff analysis, and change control executable end to end.

## 涓氬姟鑳屾櫙
Product and mapping capabilities are broad but still too shallow in several areas. This track converts those areas from partial coverage into real operational workflows.

## DoD
- ProductDiffPage is no longer an empty placeholder shell.
- Download, mapping task, conflict reason, and write-back flows are traceable.
- platform_sync_and_mapping_flow_integration passes.
- Product governance and mapping governance E2E tests pass.
- No out-of-scope edits.

## 渚濊禆鍓嶆彁
- Register shared file locks before coding.
- Stay inside the Track-04 write boundary even when sharing data with channel integration work.
- Evolve ProductDiffPage into a usable diff workflow instead of redesigning the entire product domain.

## 娴嬭瘯鍛戒护
- `cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture`
- `pnpm --filter erp-web exec playwright test e2e/shop-product-match-governance.spec.ts e2e/product-change-control.spec.ts --project=chromium`

## 浜や粯鏍煎紡
commit hash + change summary + test results + residual risks

