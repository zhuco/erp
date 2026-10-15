-- A purchase receipt fee is allocated by arrival quantity.
-- Posting the fee does not release held stock.

create table if not exists shared_receipt_fees (
  tenant_id uuid not null,
  fee_id uuid not null,
  po_id uuid not null,
  quality_id uuid not null,
  amount_cent bigint not null,
  residual_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, fee_id)
);
