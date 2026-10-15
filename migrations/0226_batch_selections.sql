-- A receipt can choose an open batch without rewriting the batch master.
-- Freeze does not erase an already recorded manufacturing use.

create table if not exists shared_batch_selections (
  tenant_id uuid not null,
  selection_id uuid not null,
  item_code text not null,
  batch_code text not null,
  as_of text not null,
  qty_micros bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, selection_id)
);
