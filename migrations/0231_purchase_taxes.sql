-- Purchase tax is the sum of source-line tax for one revision.
-- Closing an order copies the open snapshot. It does not overwrite it.

create table if not exists shared_purchase_taxes (
  tenant_id uuid not null,
  tax_id uuid not null,
  revision_id uuid not null,
  tax_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, tax_id)
);
