-- Trade document quantities use the shared six-digit scale.
-- Legacy integer 1 is stored as 1000000, not 1000.

create table if not exists trade_qty_lines (
  tenant_id uuid not null,
  document_id uuid not null,
  kind text not null,
  qty_micros bigint not null,
  unit_price_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_id, kind)
);
