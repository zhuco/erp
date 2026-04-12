-- 0006: SKU kind + kit BOM

alter table skus
  add column if not exists sku_kind text not null default 'NORMAL';

create table if not exists kit_boms (
  tenant_id uuid not null references tenants(tenant_id),
  kit_sku_id uuid not null references skus(sku_id),
  component_sku_id uuid not null references skus(sku_id),
  qty bigint not null,
  primary key (tenant_id, kit_sku_id, component_sku_id)
);
