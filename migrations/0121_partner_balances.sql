-- Confirmed partner balances. Advance cash is stored apart from open AR/AP.
create table if not exists partner_balances (
  tenant_id uuid not null,
  balance_id uuid not null,
  kind text not null,
  partner_code text not null,
  currency text not null,
  source_type text not null,
  source_id text not null,
  amount_cent bigint not null,
  settled_cent bigint not null default 0,
  due_date text,
  created_at timestamptz not null default now(),
  primary key (tenant_id, balance_id)
);

create table if not exists partner_settlements (
  tenant_id uuid not null,
  settlement_id uuid not null,
  balance_id uuid not null,
  amount_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, settlement_id)
);
