# Feature Flags

## Overview

Feature flags provide runtime control over critical operations for rollback and maintenance; platform filtering flags (for example `PDD_ONLY`) are only relevant when platform integration is enabled.

---

## Supported Flags

| Key | Scope | Description | Use Case |
|-----|-------|-------------|----------|
| `READ_ONLY` | GLOBAL/TENANT/SHOP | Block all write operations | Maintenance, debugging, rollback |
| `ORDER_SYNC` | TENANT/SHOP | Enable order synchronization | Gradual feature rollout |
| `SHIPPING_WRITE` | TENANT/SHOP | Allow shipping operations | Disable shipping during incidents |
| `INVENTORY_WRITE` | TENANT/SHOP | Allow inventory adjustments | Prevent inventory corruption |
| `PRINTING` | TENANT/SHOP | Allow print job creation | Disable printing during printer issues |
| `AFTERSALES_WRITE` | TENANT/SHOP | Allow after-sales modifications | Freeze after-sales during migration |
| `PDD_ONLY` | GLOBAL | Only allow PDD platform | Emergency platform isolation |

---

## Scope Hierarchy

Flags are checked in this order:

1. **SHOP** scope (if shop_id provided) - Most specific
2. **TENANT** scope - Tenant-level override
3. **GLOBAL** scope - System-wide default

**Example**: If `READ_ONLY` is enabled at GLOBAL scope but disabled at TENANT scope for tenant A, tenant A can write while others cannot.

---

## Database Schema

```sql
CREATE TABLE feature_flags (
  tenant_id UUID NOT NULL,
  scope TEXT NOT NULL DEFAULT 'GLOBAL',  -- GLOBAL, TENANT, SHOP
  shop_id UUID,  -- NULL for GLOBAL/TENANT
  key TEXT NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
);
```

---

## SQL Examples

### Enable READ_ONLY Globally

```sql
-- Block all writes system-wide
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES (
  '00000000-0000-0000-0000-000000000000'::UUID,  -- Dummy UUID for GLOBAL
  'GLOBAL', 
  'READ_ONLY', 
  true
)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true, updated_at = now();
```

### Disable Shipping for Specific Tenant

```sql
-- tenant_id = '11111111-1111-1111-1111-111111111111'
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES (
  '11111111-1111-1111-1111-111111111111'::UUID,
  'TENANT',
  'SHIPPING_WRITE',
  false
)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = false, updated_at = now();
```

### Enable PDD_ONLY (Emergency Platform Isolation)

```sql
-- Only allow PDD platform requests, reject taobao/jd/douyin
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES (
  '00000000-0000-0000-0000-000000000000'::UUID,
  'GLOBAL',
  'PDD_ONLY',
  true
)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true, updated_at = now();
```

### Disable Inventory Writes for Specific Shop

```sql
-- tenant_id = '11111111...', shop_id = '22222222...'
INSERT INTO feature_flags (tenant_id, scope, shop_id, key, enabled)
VALUES (
  '11111111-1111-1111-1111-111111111111'::UUID,
  'SHOP',
  '22222222-2222-2222-2222-222222222222'::UUID,
  'INVENTORY_WRITE',
  false
)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = false, updated_at = now();
```

### Check Current Flags

```sql
-- View all enabled flags
SELECT 
  scope, 
  tenant_id, 
  shop_id, 
  key, 
  enabled,
  created_at,
  updated_at
FROM feature_flags
WHERE enabled = true
ORDER BY scope DESC, created_at DESC;
```

### Disable All Flags for Tenant

```sql
UPDATE feature_flags
SET enabled = false, updated_at = now()
WHERE tenant_id = '11111111-1111-1111-1111-111111111111'::UUID;
```

---

## API Guards

Feature flags are checked at these entry points:

### erp-api

| Endpoint | Flags Checked | Behavior |
|----------|---------------|----------|
| `POST /ship` | `READ_ONLY`, `SHIPPING_WRITE` | Returns 403 with error message |
| `POST /inventory/adjust` | `READ_ONLY`, `INVENTORY_WRITE` | Returns 403 with error message |
| `POST /inventory/reserve` | `READ_ONLY`, `INVENTORY_WRITE` | Returns 403 with error message |
| `POST /inventory/release` | `READ_ONLY`, `INVENTORY_WRITE` | Returns 403 with error message |
| `POST /print-jobs` | `READ_ONLY`, `PRINTING` | Returns 403 with error message |
| `POST /after-sales/*` | `READ_ONLY`, `AFTERSALES_WRITE` | Returns 403 with error message |
| `POST /webhook/{platform}` | `PDD_ONLY` | Returns 400 for non-PDD platforms |

**Error Response Example**:

```json
{
  "error": "FEATURE_DISABLED",
  "message": "Shipping operations are disabled (READ_ONLY mode active)"
}
```

### erp-worker

Worker checks flags before processing events:

| Operation | Flags Checked | Behavior |
|-----------|---------------|----------|
| Shipping write | `READ_ONLY`, `SHIPPING_WRITE` | Skips operation, logs warning |
| Inventory write | `READ_ONLY`, `INVENTORY_WRITE` | Skips operation, logs warning |
| After-sales write | `READ_ONLY`, `AFTERSALES_WRITE` | Skips operation, logs warning |
| Platform filtering | `PDD_ONLY` | Skips non-PDD events, logs info |

**Log Example**:

```
WARN Skipping shipping write due to feature flag: READ_ONLY=true tenant_id=1111...
INFO Skipping non-PDD event due to PDD_ONLY mode platform=taobao
```

---

## Common Scenarios

### Emergency: Stop All Writes

```sql
-- Enable READ_ONLY globally
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('00000000-0000-0000-0000-000000000000'::UUID, 'GLOBAL', 'READ_ONLY', true)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true, updated_at = now();

-- Resume writes
UPDATE feature_flags
SET enabled = false, updated_at = now()
WHERE scope = 'GLOBAL' AND key = 'READ_ONLY';
```

### Platform Migration: PDD Only

```sql
-- Enable PDD_ONLY mode
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('00000000-0000-0000-0000-000000000000'::UUID, 'GLOBAL', 'PDD_ONLY', true)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true, updated_at = now();

-- Restore all platforms
UPDATE feature_flags
SET enabled = false, updated_at = now()
WHERE key = 'PDD_ONLY';
```

### Gradual Rollout: Enable Printing for One Tenant

```sql
-- Disable globally
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('00000000-0000-0000-0000-000000000000'::UUID, 'GLOBAL', 'PRINTING', false)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = false;

-- Enable for test tenant
INSERT INTO feature_flags (tenant_id, scope, key, enabled)
VALUES ('11111111-1111-1111-1111-111111111111'::UUID, 'TENANT', 'PRINTING', true)
ON CONFLICT (tenant_id, scope, COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID), key)
DO UPDATE SET enabled = true;
```

---

## Testing

Tests are located in `crates/erp-repo/tests/feature_flags.rs`:

### Test: READ_ONLY Blocks Writes

```rust
#[tokio::test]
async fn read_only_blocks_writes() {
    // Enable READ_ONLY flag
    // Attempt write operation
    // Assert operation is blocked
}
```

### Test: PDD_ONLY Rejects Non-PDD

```rust
#[tokio::test]
async fn pdd_only_rejects_other_platforms() {
    // Enable PDD_ONLY flag
    // Test taobao/jd/douyin -> rejected
    // Test pdd -> allowed
}
```

---

## Monitoring

### Query Active Flags

```sql
SELECT 
  scope,
  key,
  COUNT(*) as count,
  array_agg(DISTINCT tenant_id) as tenants
FROM feature_flags
WHERE enabled = true
GROUP BY scope, key
ORDER BY scope, key;
```

### Check Specific Tenant

```sql
SELECT scope, shop_id, key, enabled, updated_at
FROM feature_flags
WHERE tenant_id = '11111111-1111-1111-1111-111111111111'::UUID
ORDER BY scope, key;
```

---

## Troubleshooting

### Writes Still Blocked After Disabling Flag

**Check**:

1. Verify flag is disabled at all scopes:

   ```sql
   SELECT * FROM feature_flags WHERE key = 'READ_ONLY';
   ```

2. Check hierarchy - SHOP/TENANT might override GLOBAL

**Fix**: Disable at all scopes or delete row:

```sql
DELETE FROM feature_flags WHERE key = 'READ_ONLY';
```

### PDD_ONLY Not Blocking Other Platforms

**Check**:

1. Verify flag is enabled:

   ```sql
   SELECT * FROM feature_flags WHERE key = 'PDD_ONLY';
   ```

2. Check worker/API logs for guard execution

**Fix**: Ensure flag is GLOBAL scope:

```sql
UPDATE feature_flags
SET scope = 'GLOBAL'
WHERE key = 'PDD_ONLY';
```

---

## Performance Considerations

- **Caching**: Flags are queried per request; consider caching if needed
- **Index**: Index on `(tenant_id, scope, shop_id, key) WHERE enabled = true` for fast lookups
- **Scope checks**: 3 queries maximum per flag check (SHOP → TENANT → GLOBAL)

---

## Migration Path

### Applying Migration

```bash
# Via sqlx migrate
sqlx migrate run

# Or manually
psql -U postgres -d erp_db -f migrations/0012_feature_flags.sql
```

### Rollback

```sql
DROP TABLE IF EXISTS feature_flags;
```

---

**Last Updated**: 2026-01-08
