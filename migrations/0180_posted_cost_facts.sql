-- Cutoff reconciliation sums these posted facts. A difference does not update them.
create table if not exists shared_cost_facts (
  tenant_id uuid not null,
  fact_id uuid not null,
  period_code text not null,
  bucket text not null,
  amount_cent bigint not null,
  source_ref text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, fact_id)
);
