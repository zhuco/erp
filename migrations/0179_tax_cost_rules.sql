-- Tax-in-cost is an explicit tenant rule. Allocation does not invent the basis.
create table if not exists shared_tax_cost_rules (
  tenant_id uuid primary key,
  tax_is_cost boolean not null,
  created_at timestamptz not null default now()
);
