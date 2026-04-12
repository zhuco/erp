# Event Ordering Rules

## Overview

This document explains how the ERP system handles out-of-order events to prevent stale data from overwriting newer state. The core principle is: **newer events always win**, determined by timestamps or status priority.

---

## Orders Event Ordering

### Primary Strategy: Timestamp-Based

The `orders` table uses `platform_updated_at` for timestamp-based out-of-order protection.

**Location**: `crates/erp-repo/src/orders.rs` (lines 130-164)

**Logic**:

```sql
DO UPDATE SET
  status = excluded.status,
  -- ... other fields ...
WHERE (
  excluded.platform_updated_at IS NOT NULL
  AND excluded.platform_updated_at >= COALESCE(orders.platform_updated_at, '-infinity'::timestamptz)
) OR (
  -- Fallback to status priority if no timestamp
)
```

**Behavior**:

1. **If new event has `platform_updated_at`**: Compare with existing timestamp
   - `>` (newer): Update proceeds
   - `=` (same): Update proceeds (allows re-processing same version)
   - `<` (older): **Update skipped** (prevents rollback)

2. **Special case for NULL**:
   - Existing NULL is treated as `-infinity` (oldest possible)
   - New event with NULL falls back to status priority

---

### Fallback Strategy: Status Priority

When `platform_updated_at` is unavailable (platform doesn't provide it or event lacks timestamp), the system uses **status priority** to determine event freshness.

**Status Priority Table** (higher number = more advanced state):

| Status | Priority | Description |
|--------|----------|-------------|
| `PENDING_PAYMENT` | 10 | Awaiting payment |
|`ON_HOLD` | 15 | Order on hold |
| `PENDING_SHIPMENT` | 20 | Paid, awaiting shipment |
| `PARTIALLY_SHIPPED` | 30 | Some items shipped |
| `SHIPPED` | 40 | Fully shipped |
| `COMPLETED` | 50 | Transaction complete |
| `REFUNDING` | 60 | Refund in progress |
| `REFUNDED` | 70 | Refund completed |
| `CANCELLED` | 80 | Order cancelled |
| `CLOSED` | 80 | Order closed |
| Unknown | 0 | Fallback for unknown statuses |

**Logic**:

```sql
AND (
  CASE excluded.status
    WHEN 'PENDING_PAYMENT' THEN 10
    WHEN 'ON_HOLD' THEN 15
    -- ... (see table above)
    ELSE 0
  END
) >= (
  CASE orders.status
    -- ... same mapping
  END
)
```

**Behavior**:

- Only updates if new status priority `>=` existing priority
- Prevents regression (e.g., `SHIPPED` → `PENDING_SHIPMENT`)
- Allows same-status updates (e.g., updating tracking number while `SHIPPED`)

---

## After-Sales Event Ordering

After-sales cases are synced from `order_items`, which inherit ordering protection from the parent `orders` table.

**Location**: `crates/erp-repo/src/after_sale_cases.rs`

**Flow**:

1. Worker receives after-sales event
2. Updates `order_items` in orders table (protected by order-level WHERE clause)
3. `sync_cases_for_order()` aggregates from order_items
4. Case summary updated via `refresh_case_summary()`

**Protection Mechanism**:

- **Timestamp-based**: Inherited from orders table `platform_updated_at` check
- **Item-level**: If older order event is rejected, its items won't be written
- **Aggregation**: Cases are always rebuilt from current order_items state

**Status Aggregation**:

- `refund_state`: Uses `aggregate_refund_state()` (max priority wins)
- `after_sale_state`: Uses `aggregate_after_sale_state()` (most advanced wins)

---

## Platform-Specific Considerations

### Platforms with Timestamps

Platforms that reliably provide `updated_at` or `modified` timestamps:

| Platform | Field Name | Reliability | Notes |
|----------|------------|-------------|-------|
| Taobao | `modified` | ✅ High | ISO 8601 format |
| JD | `modified_time` | ✅ High | Unix timestamp |
| PDD | `updated_at` | ⚠️ Medium | May be NULL for some events |
| Douyin | `update_time` | ✅ High | Unix timestamp (seconds) |

**Worker Mapping**:
Workers parse platform-specific timestamp fields and map them to `platform_updated_at` in `OrderProjection`.

### Platforms without Timestamps

For platforms lacking update timestamps:

1. **Status priority** is the sole protection mechanism
2. Events are assumed to arrive roughly in order
3. Out-of-order window is limited to status regressions

**Risk Mitigation**:

- Worker should set `platform_updated_at` from event receive time if platform field is missing
- Ensures some timestamp exists for comparison

---

## Tests

### Order Out-of-Order Test

**File**: `crates/erp-repo/tests/ordering_rules.rs`

**Test**: `order_out_of_order_keeps_newer_state`

**Scenario**:

1. Insert order with `platform_updated_at = 2026-01-08 10:10:00` (`SHIPPED`, `tracking_no = "TRACK-NEW"`)
2. Insert same order with `platform_updated_at = 2026-01-08 10:05:00` (`PENDING_SHIPMENT`, `tracking_no = "TRACK-OLD"`)
3. Assert final state is from newer event (step 1)

**Verification**:

```rust
assert_eq!(row.status, "SHIPPED");
assert_eq!(row.tracking_no.as_deref(), Some("TRACK-NEW"));
assert_eq!(row.platform_updated_at, Some(newer_updated));
```

### After-Sales Out-of-Order Test

**File**: `crates/erp-repo/tests/ordering_rules.rs`

**Test**: `aftersales_out_of_order_keeps_newer_state`

**Scenario**:

1. Insert order with `platform_updated_at = 2026-01-08 11:10:00` (`after_sale_state = Processing`)
2. Sync after-sales case from order
3. Insert older event with `platform_updated_at = 2026-01-08 11:05:00` (`after_sale_state = Closed`)
4. Sync after-sales case again
5. Assert final case state is `Processing` (from newer event)

**Verification**:

```rust
assert_eq!(final_case.after_sale_state, AfterSaleState::Processing);
```

---

## Implementation Details

### Upsert Logic (`orders.rs`)

```rust
sqlx::query(
    r#"
    INSERT INTO orders(..., platform_updated_at, ...)
    VALUES (..., $29, ...)
    ON CONFLICT (tenant_id, order_month, platform, platform_order_id)
    DO UPDATE SET
        status = excluded.status,
        -- ... all updatable fields ...
        platform_updated_at = COALESCE(excluded.platform_updated_at, orders.platform_updated_at),
        updated_at = NOW()
    WHERE (
        excluded.platform_updated_at IS NOT NULL
        AND excluded.platform_updated_at >= COALESCE(orders.platform_updated_at, '-infinity'::timestamptz)
    ) OR (
        excluded.platform_updated_at IS NULL
        AND (
            CASE excluded.status
                WHEN 'PENDING_PAYMENT' THEN 10
                -- ... priority mapping ...
            END
        ) >= (
            CASE orders.status
                -- ... priority mapping ...
            END
        )
    )
    "#,
)
.bind(order.platform_updated_at)
// ...
.execute(&mut *tx)
.await?
.rows_affected() > 0;
```

**Key Points**:

1. `WHERE` clause prevents update if conditions not met
2. `.rows_affected() > 0` indicates whether update was applied
3. If `rows_affected() == 0`, event was stale and skipped
4. Order items are only replaced if parent order was updated (`if order_applied { ... }`)

### Worker Integration

Workers must ensure `platform_updated_at` is populated:

```rust
// Example from worker
let order = OrderProjection {
    // ... other fields ...
    platform_updated_at: event.modified.or_else(|| Some(Utc::now())),
    // ...
};

let order_id = orders_repo.upsert_order_projection(&order).await?;
```

---

## Database Schema

### Orders Table

```sql
CREATE TABLE orders (
  -- ... partition keys ...
  platform_updated_at TIMESTAMPTZ,  -- Allows NULL (treated as oldest)
  -- ... other columns ...
);
```

**Constraints**:

- `platform_updated_at` is **nullable**
- NULL is treated as `-infinity` in comparisons
- Always use `COALESCE(platform_updated_at, '-infinity'::timestamptz)` in WHERE clauses

---

## Troubleshooting

### Symptom: Order state regresses

**Check**:

1. Is `platform_updated_at` being set by worker?

   ```sql
   SELECT platform_order_id, platform_updated_at, status, updated_at
   FROM orders
   WHERE platform = 'taobao' AND platform_order_id = 'XXX';
   ```

2. Are events arriving with older timestamps?
   - Check worker logs for event timestamps
   - Verify platform API returns reliable timestamps

**Fix**:

- Ensure worker maps platform timestamp fields correctly
- If platform doesn't provide timestamp, use event receive time

### Symptom: After-sales case doesn't update

**Check**:

1. Did parent order update get applied?

   ```sql
   SELECT o.status, o.platform_updated_at, o.updated_at
   FROM orders o
   JOIN order_index i USING (tenant_id, order_id, order_month)
   WHERE i.tenant_id = $1 AND i.order_id = $2;
   ```

2. Does `order_items` have latest state?

   ```sql
   SELECT line_id, after_sale_state, refund_state
   FROM order_items oi
   JOIN order_index i USING (tenant_id, order_id, order_month)
   WHERE i.tenant_id = $1 AND i.order_id = $2;
   ```

**Fix**:

- If order was rejected (stale), items won't update
- Re-sync case won't help if source items are unchanged

### Symptom: Status priority not working

**Check**:

1. Is `platform_updated_at` NULL for both events?
2. What are the status values?

   ```sql
   SELECT status, platform_updated_at
   FROM orders
   WHERE ...;
   ```

**Fix**:

- Verify status strings match exactly (case-sensitive in WHERE clause)
- Unknown statuses get priority 0 (always lose)

---

## Performance Considerations

### Index Usage

The WHERE clause in upsert uses indexed columns:

- Primary key: `(tenant_id, order_month, platform, platform_order_id)`
- `platform_updated_at` comparison doesn't require index (only after conflict)

### Rows Affected Check

Checking `rows_affected()` adds minimal overhead:

- Prevents unnecessary item replacement
- Saves database writes and transaction time

---

## Future Enhancements

### Proposed: Event Sequence Numbers

For platforms without reliable timestamps, consider:

```sql
ALTER TABLE orders ADD COLUMN event_sequence BIGINT;
```

**Benefits**:

- Explicit ordering even without timestamps
- Can detect gaps (missing events)

**Considerations**:

- Platform must provide sequence numbers
- Requires migration and worker changes

---

## Summary

| Mechanism | Orders | After-Sales |
|-----------|--------|-------------|
| **Primary** | `platform_updated_at` >= existing | Inherited from orders |
| **Fallback** | Status priority (numeric comparison) | N/A (uses order items) |
| **NULL handling** | Treated as oldest (`-infinity`) | N/A |
| **Test coverage** | `order_out_of_order_keeps_newer_state` | `aftersales_out_of_order_keeps_newer_state` |

**Key Principle**: Newer events always win. Older events are silently skipped (`rows_affected() == 0`).

---

**Last Updated**: 2026-01-08
