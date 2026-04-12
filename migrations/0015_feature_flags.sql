-- Feature Flags Table
-- Allows runtime toggling of features for tenant/shop/global scopes

CREATE TABLE IF NOT EXISTS feature_flags (
  tenant_id UUID NOT NULL,
  scope TEXT NOT NULL DEFAULT 'GLOBAL',  -- GLOBAL, TENANT, SHOP
  shop_id UUID,  -- NULL for GLOBAL/TENANT scope
  key TEXT NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT false,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_feature_flags_unique
  ON feature_flags (
    tenant_id,
    scope,
    COALESCE(shop_id, '00000000-0000-0000-0000-000000000000'::UUID),
    key
  );

CREATE INDEX idx_feature_flags_lookup 
  ON feature_flags (tenant_id, scope, shop_id, key) 
  WHERE enabled = true;

COMMENT ON TABLE feature_flags IS 'Runtime feature toggles for rollback and platform filtering';
COMMENT ON COLUMN feature_flags.scope IS 'GLOBAL (all tenants), TENANT (specific tenant), SHOP (specific shop)';
COMMENT ON COLUMN feature_flags.key IS 'READ_ONLY, ORDER_SYNC, SHIPPING_WRITE, INVENTORY_WRITE, PRINTING, AFTERSALES_WRITE, PDD_ONLY';
