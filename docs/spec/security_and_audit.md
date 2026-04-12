# Security, RBAC, and Audit Baseline

## Scope
This document summarizes the SaaS security baseline for the multi-tenant ERP:

- Tenant isolation and query scoping
- RBAC permissions, roles, and users
- Data scope access for shops/warehouses
- Audit logging of state changes
- Dead letter queue (DLQ) and replay
- Trace IDs and metrics endpoints

## Tenant Isolation
- **JWT claims** include `tenant_id`, `user_id`, and permissions.
- **API guardrails** enforce tenant matching before data access.
- **DB schema rule**: all business tables include `tenant_id`.

Key references:
- `crates/erp-api/src/lib.rs`: `auth_middleware`, `ensure_tenant_access`
- `migrations/*`: `tenant_id` columns on business tables

## RBAC and Permissions
- **Tables**: `roles`, `role_permissions`, `user_roles`, `users`, `permissions`
- **Permissions catalog** seeded per tenant and exposed via `/admin/permissions`
- **Enforcement** via `ensure_permission` in handlers
- **Front-end** menu/button gating via permission guard

Key references:
- `migrations/0001_init.sql`, `migrations/0024_permissions_catalog.sql`
- `crates/erp-api/src/lib.rs`: `ensure_permission`
- `apps/erp-web/src/features/rbac.tsx`, `apps/erp-web/src/app/layout/menu.tsx`

## Data Scope (Shop/Warehouse)
- **Tables**: `user_shop_access`, `user_warehouse_access`
- **API**: `/admin/users/{user_id}/data-scopes`
- **Enforcement**: list filters apply scoped `shop_ids`/`warehouse_ids`

Key references:
- `migrations/0020_system_management.sql`
- `crates/erp-repo/src/data_scopes.rs`
- `crates/erp-api/src/lib.rs`: `ensure_shop_access`, `ensure_warehouse_access`

## Audit Logs
- **Table**: `audit_logs`
- **Write points**: state-changing endpoints call `record_audit_log`
- **Query**: `/admin/audit-logs` supports filters + cursor pagination

Key references:
- `migrations/0020_system_management.sql`
- `crates/erp-repo/src/audit_logs.rs`
- `crates/erp-api/src/lib.rs`: `record_audit_log`

## Dead Letter Queue (DLQ)
- **Table**: `dead_letters`
- **Worker** inserts after retries and exposes manual replay
- **API**: `/admin/dlq` (list), `/admin/dlq/{dead_letter_id}/replay`

Key references:
- `migrations/0012_dead_letters.sql`
- `crates/erp-worker/src/dlq.rs`
- `crates/erp-api/src/lib.rs`: `list_dead_letters`, `replay_dead_letter`

## Observability
- **Request/Trace IDs** propagated from HTTP → outbox → NATS → worker logs
- **Metrics**: HTTP latency, outbox backlog, worker consume stats
- **Endpoints**: `/metrics` in `erp-api` and `erp-worker`

Key references:
- `crates/erp-common/src/trace.rs`, `crates/erp-common/src/telemetry.rs`
- `crates/erp-api/src/main.rs`, `crates/erp-worker/src/main.rs`

## Admin Endpoints Summary
- `GET /admin/permissions`
- `GET/POST /admin/roles`, `PUT /admin/roles/{role_id}`
- `GET/PUT /admin/roles/{role_id}/permissions`
- `GET/POST /admin/users`, `PUT /admin/users/{user_id}`
- `GET/PUT /admin/users/{user_id}/roles`
- `GET/PUT /admin/users/{user_id}/data-scopes`
- `GET /admin/audit-logs`
- `GET /admin/dlq`, `POST /admin/dlq/{dead_letter_id}/replay`
