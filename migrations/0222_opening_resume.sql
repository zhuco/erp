-- Resuming an opening keeps the original posting. A rebuild reads each ledger on its own.

create table if not exists opening_resumes (
  tenant_id uuid not null,
  resume_id uuid not null,
  opening_id uuid not null,
  posted_rows bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, resume_id)
);

create table if not exists opening_rebuilds (
  tenant_id uuid not null,
  rebuild_id uuid not null,
  opening_id uuid not null,
  qty_micros bigint not null,
  cost_cent bigint not null,
  partner_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, rebuild_id)
);
