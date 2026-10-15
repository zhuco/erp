-- A release gate is bound to one artifact SHA. A banned package or a missing license blocks it.

create table if not exists release_gate_scans (
  tenant_id uuid not null,
  scan_id uuid not null,
  artifact_sha text not null,
  package_name text not null,
  license_name text not null default '',
  result_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, scan_id)
);

create table if not exists release_gates (
  tenant_id uuid not null,
  gate_id uuid not null,
  artifact_sha text not null,
  status text not null,
  scanned_count bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, gate_id)
);
