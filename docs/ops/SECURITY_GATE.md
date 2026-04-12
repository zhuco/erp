# SECURITY_GATE

## Scope

This document enumerates externally exposed ERP API routes, their RBAC permissions, and tenant
isolation requirements.

## Tenant isolation

All `/v1/tenants/:tenant_id/*` routes enforce tenant isolation via `ensure_tenant_access`:
tokens must match the path tenant (unless `is_admin` is true). Mismatches return `403`.

## Public routes (no auth)

| Method | Route | Auth |
| --- | --- | --- |
| GET | `/healthz` | public |
| POST | `/auth/login` | public |

## RBAC routes

Raw endpoints require dedicated `*.read_raw` permissions separate from read access.
Orders APIs also require `orders.read`/`orders.write` alongside `orders.read_raw` where noted.

| Method | Route | Permission |
| --- | --- | --- |
| GET | `/v1/tenants/:tenant_id/orders/:order_id/raw` | `orders.read`, `orders.read_raw` |
| POST | `/v1/tenants/:tenant_id/orders/:order_id/ship` | `orders.write`, `shipping.write` |
| GET | `/v1/tenants/:tenant_id/shipments` | `shipping.read` |
| POST | `/v1/tenants/:tenant_id/shipments` | `orders.write`, `shipping.write` |
| GET | `/v1/tenants/:tenant_id/after-sale-cases` | `aftersales.read` |
| GET | `/v1/tenants/:tenant_id/after-sale-cases/:case_id` | `aftersales.read` |
| GET | `/v1/tenants/:tenant_id/after-sale-cases/:case_id/raw` | `aftersales.read_raw` |
| GET | `/v1/tenants/:tenant_id/skus` | `skus.read` |
| POST | `/v1/tenants/:tenant_id/skus` | `skus.write` |
| PUT | `/v1/tenants/:tenant_id/skus/:sku_id` | `skus.write` |
| GET | `/v1/tenants/:tenant_id/warehouses` | `inventory.read` |
| GET | `/v1/tenants/:tenant_id/inventory` | `inventory.read` |
| POST | `/v1/tenants/:tenant_id/inventory/adjustments` | `inventory.write` |
| POST | `/v1/tenants/:tenant_id/inventory/reservations` | `inventory.write` |
| POST | `/v1/tenants/:tenant_id/inventory/releases` | `inventory.write` |
| GET | `/v1/tenants/:tenant_id/print-devices` | `print_devices.read` |
| POST | `/v1/tenants/:tenant_id/print-devices` | `print_devices.write` |
| POST | `/v1/tenants/:tenant_id/print-devices/register` | `print_devices.write` |
| PUT/PATCH | `/v1/tenants/:tenant_id/print-devices/:device_id` | `print_devices.write` |
| GET | `/v1/tenants/:tenant_id/print-templates` | `print_templates.read` |
| POST | `/v1/tenants/:tenant_id/print-templates` | `print_templates.write` |
| PUT | `/v1/tenants/:tenant_id/print-templates/:template_id` | `print_templates.write` |
| GET | `/v1/tenants/:tenant_id/print-jobs` | `print_jobs.read` |
| POST | `/v1/tenants/:tenant_id/print-jobs` | `print_jobs.write` |
| PUT | `/v1/tenants/:tenant_id/print-jobs/:job_id` | `print_jobs.write` |
| GET | `/v1/tenants/:tenant_id/channel-sku-mappings` | `channel_sku_mappings.read` |
| POST | `/v1/tenants/:tenant_id/channel-sku-mappings` | `channel_sku_mappings.write` |
| PUT | `/v1/tenants/:tenant_id/channel-sku-mappings/:mapping_id` | `channel_sku_mappings.write` |
| POST | `/v1/tenants/:tenant_id/fin/statements/import` | `finance.write` |
| POST | `/v1/tenants/:tenant_id/fin/reconcile/run` | `finance.write` |
| GET | `/v1/tenants/:tenant_id/fin/reconcile/jobs` | `finance.read` |
| GET | `/v1/tenants/:tenant_id/fin/reconcile/items` | `finance.read` |
| POST | `/v1/tenants/:tenant_id/fin/payment-reconcile` | `finance.write` |
| GET | `/v1/tenants/:tenant_id/fin/payment-reconcile/jobs` | `finance.read` |
| GET | `/v1/tenants/:tenant_id/fin/payment-reconcile/items` | `finance.read` |
| GET | `/v1/tenants/:tenant_id/fin/ap-docs` | `finance.read` |
| GET | `/v1/tenants/:tenant_id/fin/ap-docs/:ap_id` | `finance.read` |
| POST | `/v1/tenants/:tenant_id/fin/ap-docs/:ap_id/mark-paid` | `finance.write` |
| POST | `/v1/tenants/:tenant_id/fin/ap-docs/:ap_id/close` | `finance.write` |
| POST | `/v1/tenants/:tenant_id/fin/invoices/apply` | `finance.write` |
| GET | `/v1/tenants/:tenant_id/fin/invoices` | `finance.read` |
| GET | `/v1/tenants/:tenant_id/fin/profit` | `finance.read` |
