-- 0041: WMS outbound batch collection

create table if not exists shipment_sku_batches (
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id),
  sku_id uuid not null references skus(sku_id),
  batch_id uuid not null references sku_batches(batch_id),
  batch_no text not null,
  qty bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, shipment_id, sku_id, batch_id)
);

create index if not exists idx_shipment_sku_batches_tenant_shipment
  on shipment_sku_batches(tenant_id, shipment_id);
create index if not exists idx_shipment_sku_batches_tenant_sku
  on shipment_sku_batches(tenant_id, sku_id, batch_id);
