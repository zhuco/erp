-- A consignment return cites the original settlement and owner.
-- It does not increase another owner's stock at the same location.

create table if not exists shared_consignment_returns (
  tenant_id uuid not null,
  return_id uuid not null,
  settlement_id uuid not null,
  owner_code text not null,
  qty_micros bigint not null,
  fee_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, return_id)
);
