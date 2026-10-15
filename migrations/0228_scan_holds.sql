-- A second scan of the same code is held. It does not submit a document.

create table if not exists shared_scan_holds (
  tenant_id uuid not null,
  hold_id uuid not null,
  code text not null,
  kind text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, hold_id)
);
