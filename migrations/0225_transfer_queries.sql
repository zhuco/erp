-- A partial arrival can be queried without receiving the remainder.
-- A sale during a count does not rewrite the count sheet.

create table if not exists shared_transfer_queries (
  tenant_id uuid not null,
  query_id uuid not null,
  transfer_id uuid not null,
  issued_qty bigint not null,
  received_qty bigint not null,
  open_qty bigint not null,
  loss_qty bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, query_id)
);
