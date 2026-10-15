-- Assembly history stays when a component is removed or replaced.
-- Cost, old-part destination, and warranty are separate fields.

create table if not exists assembly_links (
  tenant_id uuid not null,
  link_id uuid not null,
  finished_serial text not null,
  component_serial text not null,
  status text not null,
  action_time text not null,
  source_action text not null,
  version_no integer not null,
  removed_at text,
  created_at timestamptz not null default now(),
  primary key (tenant_id, link_id)
);

create table if not exists service_replacements (
  tenant_id uuid not null,
  replacement_id uuid not null,
  finished_serial text not null,
  old_serial text not null,
  new_serial text not null,
  action_time text not null,
  cost_cent bigint not null,
  destination text not null,
  warranty text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, replacement_id)
);
