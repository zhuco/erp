# Industrial ERP Large-File Modularization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split and govern the four oversized artifacts (`openapi/erp_api_v1.yaml`, `crates/erp-api/src/lib.rs`, `crates/erp-api/src/orders.rs`, `crates/erp-worker/src/main.rs`) without behavioral regression and with production-safe rollout controls.

**Architecture:** Use a two-layer strategy. Layer 1 keeps external contracts stable by preserving existing output paths and runtime behavior. Layer 2 introduces source-level modularization by domain and responsibility, plus semantic CI gates (not text-format-dependent gates). Execute in phased increments with hard stop/go checkpoints.

**Tech Stack:** Rust (axum/sqlx), OpenAPI 3.0.3, Node.js scripts, pnpm workspace, openapi-typescript.

---

## Scope And Non-Goals

- In scope:
- Decompose OpenAPI source into maintainable fragments while preserving `openapi/erp_api_v1.yaml` as generated artifact.
- Refactor API and worker large files into bounded modules.
- Replace brittle OpenAPI text gate with semantic gate.
- Add regression verification and rollback playbook.

- Out of scope:
- Functional feature changes.
- Data model redesign.
- Permission model redesign.

## Baseline File Map (Current)

- `F:/erp/openapi/erp_api_v1.yaml` (single large spec source and output).
- `F:/erp/crates/erp-api/src/lib.rs` (router + DTO + handler + app state mixed).
- `F:/erp/crates/erp-api/src/orders.rs` (query/command/review/shipment logic mixed).
- `F:/erp/crates/erp-worker/src/main.rs` (bootstrap + event routing + handlers mixed).
- `F:/erp/apps/erp-web/package.json` (`generate:api` consumes `../../openapi/erp_api_v1.yaml`).
- `F:/erp/scripts/verify_openapi_gate_v2.mjs` (line-splitting text gate).

## Target File Map (After)

- OpenAPI source-of-truth:
- Create: `F:/erp/openapi/src/openapi.yaml`
- Create: `F:/erp/openapi/src/paths/*.yaml` (by domain)
- Create: `F:/erp/openapi/src/components/{schemas,parameters,responses,securitySchemes}/*.yaml`
- Keep output: `F:/erp/openapi/erp_api_v1.yaml` (bundled artifact only)

- OpenAPI tooling:
- Create: `F:/erp/scripts/openapi_bundle.mjs`
- Modify: `F:/erp/scripts/verify_openapi_gate_v2.mjs` (semantic parse)
- Modify: `F:/erp/package.json` (new `openapi:*` scripts)
- Modify: `F:/erp/apps/erp-web/package.json` (keep path contract unchanged)

- API decomposition:
- Create: `F:/erp/crates/erp-api/src/app_state.rs`
- Create: `F:/erp/crates/erp-api/src/router.rs`
- Create: `F:/erp/crates/erp-api/src/handlers/orders/{mod.rs,query.rs,command.rs,review.rs,shipment.rs}`
- Create: `F:/erp/crates/erp-api/src/handlers/{planning,marketing,sales_orders}/...`
- Modify: `F:/erp/crates/erp-api/src/lib.rs` (reduce to wiring + exports)
- Modify: `F:/erp/crates/erp-api/tests/*` (regression assertions only)

- Orders domain decomposition:
- Create: `F:/erp/crates/erp-api/src/orders/{mod.rs,query.rs,command.rs,review.rs,shipment.rs,dto.rs,mapper.rs}`
- Modify: `F:/erp/crates/erp-api/src/orders.rs` (temporary shim or removal at end phase)

- Worker decomposition:
- Create: `F:/erp/crates/erp-worker/src/bootstrap.rs`
- Create: `F:/erp/crates/erp-worker/src/event_router.rs`
- Create: `F:/erp/crates/erp-worker/src/handlers/{inbox.rs,order.rs,profit.rs,token.rs,dlq.rs,sync_job.rs}`
- Modify: `F:/erp/crates/erp-worker/src/main.rs` (bootstrap + composition only)

## Delivery Strategy

- Phase 0: Baseline and freeze.
- Phase 1: OpenAPI split source + bundle output compatibility.
- Phase 2: OpenAPI semantic gate replacement.
- Phase 3: `erp-api/lib.rs` modularization.
- Phase 4: `erp-api/orders.rs` modularization.
- Phase 5: `erp-worker/main.rs` modularization.
- Phase 6: Hardening, rollback drills, and handover.

Each phase has go/no-go criteria. Do not start next phase if current phase fails.

## Architecture Approval Conditions

- External contract stability is mandatory.
- `openapi/erp_api_v1.yaml` remains the only generated artifact consumed by downstream tools.
- Front-end generation entrypoints keep working without callers learning new paths.
- API and worker runtime behavior must remain equivalent for permissions, audit logging, idempotency, retries, and transaction boundaries.

- Internal modularization is allowed only when it improves domain boundaries.
- `lib.rs` becomes composition root, not another shared dumping ground.
- `orders.rs` splits by business responsibility, not arbitrary technical layers.
- `main.rs` keeps bootstrap separate from event routing and per-event handlers.

- Delivery must remain reversible.
- Every phase ships with a rollback point.
- No phase may require a flag day or freeze across all teams.

## No-Go Conditions

- Any path, method, permission key, or required response status changes without explicit approval.
- Any loss of `x-persistence-required` or `x-persistence-tables` declarations on protected write APIs.
- Any worker refactor that scatters retry, dead-letter, or read-only guard behavior across unrelated modules.
- Any refactor batch too large to isolate in review or safely revert.

---

### Task 1: Create Safe Working Baseline

**Files:**
- Modify: `F:/erp/.gitignore` (optional, if temporary bundle dirs are introduced)
- Test: repository baseline commands only

- [ ] **Step 1: Capture current baseline metrics**

Run:
```bash
git -C F:/erp rev-parse --short HEAD
git -C F:/erp status --short
pnpm -C F:/erp run repo:verify
```
Expected: current baseline status and gate output recorded in task notes.

- [ ] **Step 2: Record oversized file KPIs in project notes**

Run:
```bash
node -e "const fs=require('fs');const p=['F:/erp/openapi/erp_api_v1.yaml','F:/erp/crates/erp-api/src/lib.rs','F:/erp/crates/erp-api/src/orders.rs','F:/erp/crates/erp-worker/src/main.rs'];for(const f of p){const t=fs.readFileSync(f,'utf8');console.log(f,'lines=',t.split(/\r?\n/).length,'bytes=',Buffer.byteLength(t));}"
```
Expected: metrics snapshot committed in blueprint execution log.

- [ ] **Step 3: Define stop criteria**

Write and share hard stop policy:
- Any contract diff in path/method/permission/response code not explicitly approved.
- Any failing `repo:verify`.
- Any e2e critical path regression.

- [ ] **Step 4: Commit baseline note**

Run:
```bash
git add docs/superpowers/plans/2026-04-14-industrial-erp-large-file-modularization-blueprint.md
git commit -m "docs(plan): add industrial ERP large-file modularization blueprint"
```
Expected: traceable starting point.

### Task 2: OpenAPI Source Split Skeleton (No Contract Change)

**Files:**
- Create: `F:/erp/openapi/src/openapi.yaml`
- Create: `F:/erp/openapi/src/paths/index.yaml`
- Create: `F:/erp/openapi/src/components/index.yaml`
- Modify: `F:/erp/openapi/erp_api_v1.yaml` (generated output only)

- [ ] **Step 1: Create root split spec file**

Root file structure:
```yaml
openapi: 3.0.3
info:
  title: ERP API
  version: 1.0.0
servers:
  - url: http://localhost:8080
paths:
  $ref: ./paths/index.yaml
components:
  $ref: ./components/index.yaml
```

- [ ] **Step 2: Move one pilot domain (example: reports + health) into split files**

Expected: small pilot ensures toolchain correctness before full migration.

- [ ] **Step 3: Keep bundled output path stable**

Output file MUST remain:
- `F:/erp/openapi/erp_api_v1.yaml`

- [ ] **Step 4: Verify front-end compatibility immediately**

Run:
```bash
pnpm -C F:/erp/apps/erp-web run generate:api
pnpm -C F:/erp/apps/erp-web run typecheck
```
Expected: pass with unchanged command interface.

- [ ] **Step 5: Commit pilot split**

```bash
git add openapi/src openapi/erp_api_v1.yaml
git commit -m "chore(openapi): introduce split source skeleton with stable bundled output"
```

### Task 3: Introduce OpenAPI Bundling Script And Commands

**Files:**
- Create: `F:/erp/scripts/openapi_bundle.mjs`
- Modify: `F:/erp/package.json`

- [ ] **Step 1: Add bundling script**

Script responsibilities:
- Read from `openapi/src/openapi.yaml`.
- Bundle to `openapi/erp_api_v1.yaml`.
- Fail fast on unresolved `$ref`.

- [ ] **Step 2: Add npm scripts**

Add scripts:
```json
{
  "openapi:bundle": "node scripts/openapi_bundle.mjs",
  "openapi:check": "pnpm run openapi:bundle && pnpm run repo:verify:openapi"
}
```

- [ ] **Step 3: Validate deterministic output**

Run:
```bash
pnpm -C F:/erp run openapi:bundle
git -C F:/erp diff -- openapi/erp_api_v1.yaml
pnpm -C F:/erp/apps/erp-web run generate:api
```
Expected: no unexpected drift after repeated bundle.

- [ ] **Step 4: Commit tooling**

```bash
git add scripts/openapi_bundle.mjs package.json openapi/erp_api_v1.yaml
git commit -m "build(openapi): add deterministic bundle pipeline"
```

### Task 4: Replace Text Gate With Semantic OpenAPI Gate

**Files:**
- Modify: `F:/erp/scripts/verify_openapi_gate_v2.mjs`
- Create: `F:/erp/scripts/openapi_gate_rules.json` (rule data externalized)
- Test: `F:/erp/scripts/verify_openapi_gate_v2.test.mjs` (Node test runner)

- [ ] **Step 1: Externalize verification rules**

Move hardcoded endpoint rule list to JSON data file. Keep same rule semantics.

- [ ] **Step 2: Parse YAML semantically**

Required behavior:
- Parse full OpenAPI object.
- Resolve method objects by `paths[path][method]`.
- Validate `x-permissions`, `responses`, and `x-persistence-*` fields.

- [ ] **Step 3: Add unit tests for gate**

Test cases:
- pass case for known valid endpoint.
- fail case for missing permission.
- fail case for missing response code.
- fail case for missing persistence declarations.

- [ ] **Step 4: Run gate in repo verify**

Run:
```bash
pnpm -C F:/erp run repo:verify:openapi
pnpm -C F:/erp run repo:verify
```
Expected: pass with semantic checker.

- [ ] **Step 5: Commit gate refactor**

```bash
git add scripts/verify_openapi_gate_v2.mjs scripts/openapi_gate_rules.json scripts/verify_openapi_gate_v2.test.mjs
git commit -m "build(openapi): replace text-based gate with semantic gate"
```

### Task 5: Complete OpenAPI Domain Migration

**Files:**
- Modify/Create: `F:/erp/openapi/src/paths/**/*.yaml`
- Modify/Create: `F:/erp/openapi/src/components/**/*.yaml`
- Modify: `F:/erp/openapi/src/openapi.yaml`
- Modify: `F:/erp/openapi/erp_api_v1.yaml` (bundle output)

- [ ] **Step 1: Migrate by domain in small batches**

Domain batch order:
1. `system`, `auth`, `health`.
2. `orders`, `after-sales`.
3. `wms`, `inventory`.
4. `finance`, `planning`, `reports`.

- [ ] **Step 2: After each batch run contract checks**

```bash
pnpm -C F:/erp run openapi:bundle
pnpm -C F:/erp run repo:verify:openapi
pnpm -C F:/erp/apps/erp-web run generate:api
pnpm -C F:/erp/apps/erp-web run typecheck
```

- [ ] **Step 3: Track migration completion KPIs**

KPIs:
- 100% paths loaded from split source.
- bundled output path unchanged.
- zero unresolved ref.

- [ ] **Step 4: Commit domain batches**

Commit per domain batch to keep rollback granular.

### Task 6: Refactor `erp-api/src/lib.rs` To Composition Root

**Files:**
- Create: `F:/erp/crates/erp-api/src/app_state.rs`
- Create: `F:/erp/crates/erp-api/src/router.rs`
- Create: `F:/erp/crates/erp-api/src/handlers/mod.rs`
- Modify: `F:/erp/crates/erp-api/src/lib.rs`
- Test: existing `F:/erp/crates/erp-api/tests/*.rs`

- [ ] **Step 1: Move `AppState` and constructor logic**

From `lib.rs` to `app_state.rs`:
- `AppState` struct.
- `new/new_with_nats` constructors.
- repo wiring helpers.

- [ ] **Step 2: Move router assembly**

From `lib.rs` to `router.rs`:
- `build_router` and route registration.
- middleware wiring unchanged.

- [ ] **Step 3: Keep lib surface stable**

`lib.rs` exports only:
- module declarations.
- selected public re-exports.
- startup entrypoints.

- [ ] **Step 4: Run targeted regression tests**

```bash
cargo test -p erp-api --test oms_orders -- --nocapture
cargo test -p erp-api --test shipments -- --nocapture
cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
```

- [ ] **Step 5: Commit lib modularization**

```bash
git add crates/erp-api/src/lib.rs crates/erp-api/src/app_state.rs crates/erp-api/src/router.rs crates/erp-api/src/handlers
git commit -m "refactor(api): split lib composition root into app_state and router modules"
```

### Task 7: Refactor `erp-api/src/orders.rs` By Responsibility

**Files:**
- Create: `F:/erp/crates/erp-api/src/orders/mod.rs`
- Create: `F:/erp/crates/erp-api/src/orders/{query.rs,command.rs,review.rs,shipment.rs,dto.rs,mapper.rs}`
- Modify: `F:/erp/crates/erp-api/src/orders.rs` (temporary forwarding or remove)
- Test: `F:/erp/crates/erp-api/tests/oms_orders.rs`

- [ ] **Step 1: Move DTO/request-response types to `dto.rs`**

No behavior change. Pure relocation.

- [ ] **Step 2: Move read-only endpoints and query helpers to `query.rs`**

Include list/detail/filter/pagination handlers.

- [ ] **Step 3: Move write paths to `command.rs` and shipping workflows to `shipment.rs`**

Keep operation logs and permission checks unchanged.

- [ ] **Step 4: Move review/recheck logic to `review.rs`**

Keep all review action constants and status transitions equivalent.

- [ ] **Step 5: Run focused tests**

```bash
cargo test -p erp-api --test oms_orders -- --nocapture
cargo test -p erp-api --test order_match_ship_wave_persistence_guard -- --nocapture
```

- [ ] **Step 6: Commit orders modularization**

```bash
git add crates/erp-api/src/orders crates/erp-api/src/orders.rs crates/erp-api/tests/oms_orders.rs
git commit -m "refactor(api): split orders module into query command review shipment units"
```

### Task 8: Refactor `erp-worker/src/main.rs` Into Bootstrap + Router + Handlers

**Files:**
- Create: `F:/erp/crates/erp-worker/src/bootstrap.rs`
- Create: `F:/erp/crates/erp-worker/src/event_router.rs`
- Create: `F:/erp/crates/erp-worker/src/handlers/{mod.rs,inbox.rs,order.rs,profit.rs,token.rs,dlq.rs,sync_job.rs}`
- Modify: `F:/erp/crates/erp-worker/src/main.rs`

- [ ] **Step 1: Isolate startup and dependency wiring**

Move environment/config/client init into `bootstrap.rs`.

- [ ] **Step 2: Extract event dispatch table**

Move `route_event` and event-type matching into `event_router.rs`.

- [ ] **Step 3: Move business handlers into handler modules**

One event family per file. Keep tracing + idempotency behavior unchanged.

- [ ] **Step 4: Run worker and integration checks**

```bash
cargo test -p erp-worker
cargo test -p erp-api --test platform_sync_and_mapping_flow_integration -- --nocapture
```

- [ ] **Step 5: Commit worker modularization**

```bash
git add crates/erp-worker/src/main.rs crates/erp-worker/src/bootstrap.rs crates/erp-worker/src/event_router.rs crates/erp-worker/src/handlers
git commit -m "refactor(worker): split main into bootstrap router and handler modules"
```

### Task 9: Cross-Cutting Verification And Release Gates

**Files:**
- Modify: `F:/erp/docs/ops/RELEASE_GATE_CHECKLIST_V2.md` (if needed)
- Modify: `F:/erp/docs/ops/GO_NO_GO_CHECKLIST_V2.md` (if needed)

- [ ] **Step 1: Run full repository gate**

```bash
pnpm -C F:/erp run repo:verify
pnpm -C F:/erp run release:gate
```
Expected: pass all mandatory checks.

- [ ] **Step 2: Run high-risk API tests**

```bash
cargo test -p erp-api --test finance_reconcile -- --nocapture
cargo test -p erp-api --test wms_labor_slotting_flow -- --nocapture
cargo test -p erp-api --test after_sales_exchange_profit_flow -- --nocapture
```

- [ ] **Step 3: Validate generated front-end contract**

```bash
pnpm -C F:/erp/apps/erp-web run generate:api
pnpm -C F:/erp/apps/erp-web run build
pnpm -C F:/erp/apps/erp-web run test:unit
```

- [ ] **Step 4: Prepare release note**

Include:
- module boundary changes.
- no-contract-change statement.
- rollback instructions.

### Task 10: Rollback And Contingency Drill

**Files:**
- Create: `F:/erp/docs/ops/LARGE_FILE_MODULARIZATION_ROLLBACK.md`

- [ ] **Step 1: Define rollback switches**

- OpenAPI rollback: regenerate from previous monolith file revision.
- API rollback: revert module split commits only (keep baseline tags).
- Worker rollback: revert event router extraction commit.

- [ ] **Step 2: Dry-run rollback in staging**

Run:
```bash
git revert <phase-commit-sha>
pnpm -C F:/erp run repo:verify
```
Expected: service and gate recover to pre-split baseline.

- [ ] **Step 3: Commit rollback runbook**

```bash
git add docs/ops/LARGE_FILE_MODULARIZATION_ROLLBACK.md
git commit -m "docs(ops): add modularization rollback runbook"
```

---

## Quality Gates (Must Pass)

- Contract parity:
- operation count unchanged unless explicitly planned.
- no removed/renamed path or method.
- same required permission keys for guarded endpoints.

- Build and tests:
- `pnpm run repo:verify` passes.
- `cargo test -p erp-api` critical suites pass.
- `pnpm --dir apps/erp-web run generate:api` and `typecheck` pass.

- Operability:
- logs/traces unchanged in critical flows.
- rollback procedure tested once in staging.

## RACI (Suggested)

- Responsible (implementation): API/Platform engineers.
- Accountable (acceptance): architecture owner + tech lead.
- Consulted: QA, DevOps, Security/Compliance.
- Informed: product owners for affected domains.

## Risk Register

- Risk: semantic gate migration introduces false positives.
- Mitigation: run old and new gate in parallel for 1 sprint.

- Risk: module split introduces hidden circular imports.
- Mitigation: split by vertical domain first, then shared utilities.

- Risk: team velocity dip during migration.
- Mitigation: commit by domain batch and keep each batch releasable.

## Definition Of Done

- The four target files no longer carry mixed responsibilities.
- OpenAPI editing is performed in `openapi/src/**`, and output path remains stable.
- Semantic gate replaces text gate in CI.
- Release gate and rollback drill both pass.
- Handover docs updated.

