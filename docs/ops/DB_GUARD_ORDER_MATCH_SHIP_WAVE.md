# DB Guard: Order Match Ship Wave

## Scope

This guard applies to all P0 mutation chains in industrial ERP:

- Order processing and order edit
- Product matching (auto/manual)
- Shipment confirm/cancel execution
- WMS wave ship execution

The following persistent tables are mandatory for traceability:

- `order_action_log`
- `order_status_history`
- `operation_logs`
- `audit_logs`

## Guard Strategy

Two layers are enabled together:

1. Application guard in repository write paths.
2. Database `CHECK` constraints in migration `0093_order_match_ship_wave_action_guard.sql`.

A mutation is not considered complete if it returns success without at least one persisted action row in the expected table family.

## Required Non-Blank Fields

- `order_action_log`: `action_code`, `idempotency_key`, `status`
- `order_status_history`: `to_status`, `action_code` (if provided)
- `operation_logs`: `actor_type`, `action`
- `audit_logs`: `action`, `entity_type`

## Release Gate Checks

Run before go/no-go:

```bash
cargo test -p erp-repo --lib order_action_logs::tests::ensure_non_blank_rejects_blank_values
cargo test -p erp-repo --lib order_status_history::tests::ensure_non_blank_rejects_blank_values
cargo test -p erp-repo --lib operation_logs::tests::ensure_non_blank_rejects_blank_values
cargo test -p erp-repo --lib audit_logs::tests::ensure_non_blank_rejects_blank_values
cargo test -p erp-api --test order_match_ship_wave_persistence_guard -- --nocapture
```

If `DATABASE_URL` and `TEST_DATABASE_URL` are both missing, release gate must fail with NO-GO.
`order_match_ship_wave_persistence_guard` is a mandatory gate in release verification and cannot be treated as a successful skip.

## Operational Notes

- Migration adds constraints with `NOT VALID` to avoid blocking old historical dirty rows.
- New writes are still enforced after migration.
- If historical rows need cleanup, run data repair and later validate constraints in a controlled window.
