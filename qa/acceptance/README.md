# REAL API acceptance database

## Current complete free ERP UI QA baseline (2026-10-08)

`erp_free_ui_acceptance` on the same loopback PostgreSQL port 15438 is the fixed
fresh-install UI database. Initialize/resume it with
`python qa/acceptance/free-erp-ui-migrate.py`. The script checks database, role,
address, port and an ownership marker; applies official SQL in full filename
order with one transaction per file; records each successful SHA; refuses
changed applied hashes; and never drops/reinitializes any database. The current
320 SQL files, including duplicate numeric prefixes, are all retained as
distinct filename identities. The fixed `--fresh-install` mode creates/resumes
only `erp_free_install_acceptance` with its own ownership marker and separate
fixed-purpose logs/ledger; it never drops either database. All 320 files passed
on that actual empty database, including the 12 runtime tables in 0311 without
acceptance-test CREATE. This is not populated-upgrade, backup-restore or release
qualification. The latest full backend target passed 577 functional cases, but
the original-data verifier found a fairness fixture globally closing two old
queue rows. The fixture now uses a real isolated queue; its targeted rerun and
new preservation window passed. Do not call the earlier full window protected
or infer a new full run from the repaired target; see
`free-erp-runtime-tables-oct8-repair.json`.

Create the separate owned product samples with
`python qa/acceptance/free-erp-product-web-fixture.py`; this is add-only and does
not reset the historical deterministic seed below. API `DATABASE_URL` must be
`postgresql://erp_grid_test@127.0.0.1:15438/erp_free_ui_acceptance`. The fixed QA
credential belongs only to this fresh fixture; the manifest contains no password
or token. Real browser captures are under `output/playwright/product-web` and
are reconciled by `free-erp-product-web-verify.py`. The two retained source/SQL
windows each contain 1007 hashes. See
`free-erp-product-web-oct8-repair.json` for scope and remaining gates.

`erp_shared_contract_test` is a protected partial compatibility schema for
historical backend facts; it is not the whole-application UI database.
`erp_grid_acceptance` below retains its historical migration ledger through
0101 and its old samples. Do not silently fill missing columns in either old
database to claim current complete migration or whole-UI acceptance.

## Historical grid fixture

This fixture is isolated to PostgreSQL `127.0.0.1:15438/erp_grid_acceptance` as role
`erp_grid_test`. Never point these commands at port 5432.

Apply after all files in `migrations/` have succeeded:

```powershell
& 'G:\PostgreSQL\17\bin\psql.exe' -h 127.0.0.1 -p 15438 -U erp_grid_test `
  -d erp_grid_acceptance -X -v ON_ERROR_STOP=1 -f 'G:\erp\qa\acceptance\seed.sql'
```

API settings: `DATABASE_URL=postgres://erp_grid_test@127.0.0.1:15438/erp_grid_acceptance`.
The API base URL/port is chosen by the service launcher. Login with tenant
`a1100000-0000-4000-8000-000000000001`, email `qa.acceptance@example.invalid`, and
the synthetic QA password `ErpQa!2026`. The fixture grants product read/write,
warehouse/inventory read/write, report read, and export read/write, plus one shop
and one warehouse scope. Re-running the seed restores deterministic baseline values.

The database records every successfully applied migration filename and SHA-256 in
`qa_acceptance_migration_progress`; a changed checksum must be investigated rather
than overwritten. Do not drop or reinitialize this database.

The repeatable Rust canvas smoke command is `pnpm run test:e2e`. Start the isolated
API on `127.0.0.1:15441` and build/serve WASM on `127.0.0.1:15440` first. The command
uses the locked Playwright CLI and the above synthetic tenant only; it fails if
the fixture or services are unavailable. It edits QA-P-001 through the browser,
checks API readback and restores the original value through the browser. Do not
run concurrently with manual edits to that fixture. Evidence goes to
`output/playwright/browser-smoke.json`; source/test configuration alone does not
count as a passing execution.

Order-grid acceptance uses the same synthetic identity and shop. Apply the
add-only `orders.sql` file on this same isolated database to grant only order
read/write and add twelve `QA-ORDER-*` records with two lines each. The file checks
database name, port and existing QA user before writing; conflicts do nothing, so
rerunning it does not reset saved order edits or the product acceptance fixture.
It does not start a worker, reserve inventory or send an order to a platform.

Cross-page fill uses add-only `cross-fill.sql`: twenty-four `QA-FILL-*` products
with separate identities. It enforces the same database/port/user guards and
never overwrites an existing product. This provides more than two ten-row pages
without resetting the original product/SKU fixture. Loading this fixture is not
evidence that browser filling or saving has passed.
