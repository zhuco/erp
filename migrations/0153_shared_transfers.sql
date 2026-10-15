-- A transfer is issued, then received or explicitly lost. A count sheet does not replace the balance.

create table if not exists shared_transfer_lines (
  tenant_id uuid not null,
  line_id uuid not null,
  transfer_id uuid not null,
  kind text not null,
  delta_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, line_id)
);

create table if not exists shared_count_sheets (
  tenant_id uuid not null,
  count_id uuid not null,
  cutoff_at text not null,
  counted_qty bigint not null,
  adjustment_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, count_id)
);
