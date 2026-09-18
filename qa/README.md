# 易格数据库测试

`postgres-grid/` 是隔离的 PostgreSQL 17 测试数据目录，**不是构建缓存**，不得被缓存清理脚本删除。固定目录复用，不按任务新建集群。测试进程只监听 `127.0.0.1:15438`，使用本地测试角色 `erp_grid_test`，不连接已有 ERP/Venue 数据库或启动系统 PostgreSQL 服务。

仓库测试 `product_grid_postgres_full_query_then_page_and_tenant_isolation` 默认忽略；仅显式设置 `ERP_GRID_TEST_DATABASE_URL` 后，通过统一入口 `scripts/Invoke-ErpBuild.ps1 -Action test -Scope backend -TestFilter product_grid -IncludeIgnored` 执行。没有数据库不能算通过。

测试使用单连接池创建会话临时 `products` 表，种入 10,000 条及隔离/删除夹具，断开后临时表由 PostgreSQL 释放。该测试验证真实 SQL 查询，不代表整个 ERP 迁移、真实账户登录或生产数据验收。

商品动作测试 `product_actions_postgres_copy_cas_replay_concurrency_and_audit_atomicity` 使用最多四个连接，在本次创建的 `grid_action_test_<UUID>` schema 内应用相关商品迁移 DDL；search_path 不含 public。只向该 schema 写夹具并验证并发重放、CAS 与审计回滚，成功后删除该测试 schema。失败遗留 schema 是排障数据，不属于构建缓存；不得自动扩大清理范围。统一入口可用 `-TestFilter product_ -IncludeIgnored` 同时验证相关查询与动作库测试。

本机二进制位于 `G:/PostgreSQL/17/bin`；集群日志为 `qa/postgres-grid.log`。启停必须确认 `-D` 为此目录，只能操作本测试集群。初始化是一次性的；已存在目录应先检查，不覆盖、不重新初始化。测试角色及 trust 认证只用于此回环地址测试集群，不用于业务部署。

## 动作持久化验收

原生测试随 `scripts/Invoke-ErpBuild.ps1 -Action test` 执行。SQLite 用测试自己创建的临时目录；用例结束关闭连接并释放自己的夹具，不连接用户 `%LOCALAPPDATA%/ErpGrid` 数据库。进程中断用例仅终止由该用例启动的子进程，验证未提交事务回滚，不能据此声称完成真实硬件断电验收。

浏览器独立夹具启动命令为 `node scripts/serve-action-storage-qa.mjs`，只监听 `127.0.0.1:15439`，仅暴露测试 HTML 和实际 IndexedDB 适配模块，不提供仓库目录浏览或 ERP API。固定测试 profile 为 `qa/browser-action-storage/`；该目录是浏览器测试数据，不属于构建缓存，保留用于关闭/重开测试，不自动清理。

Windows 可通过 `npx.cmd --yes --package @playwright/cli playwright-cli -s=erp-grid-storage open http://127.0.0.1:15439/ --browser msedge --persistent --profile G:/erp/qa/browser-action-storage` 打开；先 snapshot，再按最新引用点击 `Run storage checks`。成功后关闭这个命名浏览器，用同一 profile 重开并点击 `Verify persisted checkpoint`。夹具仅清理此 QA origin 中以 `qa-` 开头的自有记录，不删除数据库、profile 或其他 scope；测试不使用真实用户身份、密码或商品。

0.20 实测 Edge 152 的存储功能及浏览器重开均通过；重开证据保存到 `output/playwright/action-storage-reopen.png`。favicon 404 属于夹具缺少图标，不是 IndexedDB 错误。此测试只验收浏览器存储适配，完整 Rust/WASM 页面、真实商品 API 和性能仍须后续验证。


## Rich XLSX regression artifact

The `rich_workbook_packages_image_relationships_and_styles` Rust test writes its workbook only when `ERP_GRID_EXPORT_QA_PATH` is explicitly set. Use the normal `scripts/Invoke-ErpBuild.ps1 -Action test -TestFilter export` build entry. `qa/export-rich.xlsx` is an isolated fixture with no business data. Independently inspect it with `python qa/verify-export-rich.py qa/export-rich.xlsx` (requires openpyxl and Pillow). The checker reads only: XML parsing, duplicate ZIP parts, PNG pixel decode, workbook styles, frozen header and image anchor bounds. This is file interoperability evidence, not an Excel desktop UI or printer acceptance test.

## Export download bridge

Run `node qa/download-web.test.mjs` for the actual JavaScript bridge with mocked browser file/fetch APIs. Checks cover >8 MiB streaming, incomplete-body abort, HTTP authorization status, bounded Blob fallback and cancellation before file publication. These checks do not constitute real browser picker or authenticated ERP acceptance. Native temporary-file download tests use the guarded build entry with `-Action test -TestFilter download`; server stream tests use `-Scope backend -TestFilter export_stream`.

## Product query export QA

`-Scope backend -TestFilter product_export -IncludeIgnored` checks the API contract and temporary-table job/audit atomicity. `-Scope backend -TestFilter product_grid_export -IncludeIgnored` checks real cursor binding, 1000+1 batching and repeatable-read visibility using the same explicit `ERP_GRID_TEST_DATABASE_URL`. Use the existing local QA cluster described above. `-Scope worker -TestFilter product_` checks CSV escaping and bounded writes. All Rust commands go through `scripts/Invoke-ErpBuild.ps1`; the worker scope shares the fixed cache and lock. These fixtures do not prove real account UI-to-worker-to-file acceptance or million-row performance. `worker-clippy.log` records existing adapter lint blockers encountered during 0.41.

## Remote product find QA

Use `scripts/Invoke-ErpBuild.ps1 -Action test -Scope backend -TestFilter find -IncludeIgnored` with the explicit local QA database URL. The repository fixture checks ordered visible-cell traversal, live row offset, forward/backward wrapping, tenant/deleted exclusion, literal wildcards and Chrono-compatible timestamp fractions. API tests reject foreign tenants, denied permissions and invalid text before database access. UI decoder tests use `-TestFilter remote_view::find`; they do not replace real browser/desktop keyboard or IME acceptance. Product export permission compatibility is checked separately with `-Scope backend -TestFilter product_export_requires`.


## Frozen product selection export QA

Use `-Scope backend -TestFilter product_selection_export -IncludeIgnored` with the explicit local QA URL above. The test uses connection-local temporary tables and verifies actor binding, exclusions, retained data after original selection deletion, request identity replay, missing held-row rejection and DONE cleanup. `-TestFilter frozen_export` covers request serialization/retry and HTTP 410 recovery; API validation uses `-Scope backend -TestFilter product_export`. Worker integration compiles with `-Scope worker -Action check`; existing adapter Clippy blockers remain recorded separately. Apply migration 0101 before starting API/worker with this feature. These checks do not constitute authenticated UI-to-worker-to-file acceptance.


## Explicit checked product export QA

`-Scope backend -TestFilter checked_export -IncludeIgnored` covers the API source validation and repository ID/sort/tenant checks using the explicit QA URL above. The database fixture verifies complete held rows, sorting, one audit for an exact retry, unchanged exported fields after product mutation, and atomic rejection of a foreign ID. `-Scope repo -TestFilter product_selection_export -IncludeIgnored` checks the directly shared frozen-selection helpers without rebuilding API. The repo scope uses the same fixed cache, lock and resource admission. UI coverage uses `-TestFilter checked_export`. No new migration beyond 0101 is required. Actual account desktop/Web end-to-end remains a separate acceptance item.


## SKU remote find QA

Use `-Scope backend -TestFilter sku_find -IncludeIgnored` with the explicit local QA URL. Tests cover API rejection before database access, nine-column validation and real SQL traversal under parent/tenant/deleted scope, literal wildcard matching, money/KIT cost, boolean and UTC precision. The directly shared SKU ordering regression uses `-Scope repo -TestFilter sku_grid::tests -IncludeIgnored`. UI checks use `-TestFilter find` and `-TestFilter sku_query_reset`, covering resource identity, anchor preservation, response decoding, exact row/column focus and stale pending-focus cancellation. Native/WASM Clippy passed; real account keyboard/IME acceptance remains separate.


## Frozen selection find QA

Use `-Scope backend -TestFilter frozen_find -IncludeIgnored` with the explicit isolated QA URL. Tests cover read-only route classification, access rejection, frozen-only request validation, snapshot sort/column traversal/wrapping and actor/expiry binding; no live products table is used by the SQL fixture. UI checks use `-TestFilter remote_view::find` and `-TestFilter frozen_find_transition`, covering token identity, 410 classification, existing anchors/decoding and cancellation when the frozen source changes. The frozen find searches displayed rows including exclusions, without changing selection. Actual desktop/Web acceptance remains separate.

## Inventory aggregate QA

The aggregate endpoint reads current inventory balances across the authorized warehouse scope, independently of product paging. Run the new `inventory_grid_report` repository tests using `-Scope repo -TestFilter inventory_grid_report -IncludeIgnored` with the explicit local QA URL above. The fixture must cover tenant and warehouse isolation, quantity and exact cost summaries, absent costs and distinct null categories. API checks use the new inventory aggregate handler test module; UI conversion and lifecycle checks use the inventory report module. These checks do not replace real account desktop/Web interaction acceptance, historical inventory reporting or large-dataset performance measurements. No business database migration is needed for this read-only endpoint.

## Inventory report export QA

Use `scripts/Invoke-ErpBuild.ps1 -Action test -TestFilter remote_view::inventory_report` for the directly affected report module. The export tests verify complete leaf groups with one grand total despite collapsed UI rows, dynamic pivot and visible-column order, exact quantity/cost formatting, and an actual generated XLSX decoded with calamine. File delivery is reused; these unit/integration checks do not constitute actual browser downloads or printer acceptance. Native and WASM Clippy passed for 0.49; no database changes or SQL test rerun were required for this UI-only batch.

## Remote warehouse tree QA

Use `scripts/Invoke-ErpBuild.ps1 -Action test -Scope backend -TestFilter warehouse_tree -IncludeIgnored` with the explicit isolated QA URL to cover pure-read root/child keyset SQL, permissions and warehouse scope. Use grid `-TestFilter tree` for the changed TreeModel and its direct fixture/remote UI consumers. Continuation is live keyset traversal, not a frozen multi-request snapshot; tests must check ordered continuation, failure retry, duplicate batches, stable typed node identity and old-generation rejection. No business migrations are introduced. Completion evidence is recorded after integration; actual desktop/browser mouse/keyboard usage remains a separate acceptance item.

0.50 integration result: backend warehouse_tree filter passed 4 checks including the isolated PostgreSQL fixture; grid tree filter passed 17 checks across core, direct UI consumers and renderer. Native/WASM Clippy, affected-file formatting and OpenAPI gate passed. The only Clippy correction grouped decoder progress parameters; no database or historical full suite rerun was needed. The isolated cluster was stopped. Live account interaction and large-scale performance remain separate acceptance work.
