-- A sampling decision cites a stored rule version. A later version does not rewrite it.
-- A completed recall freeze posts unavailable stock. Notice stays a separate result.

create table if not exists shared_sampling_rules (
  tenant_id uuid not null,
  rule_id uuid not null,
  rule_code text not null,
  rule_version integer not null,
  sample_size bigint not null,
  accept_number bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, rule_id),
  unique (tenant_id, rule_code, rule_version)
);
